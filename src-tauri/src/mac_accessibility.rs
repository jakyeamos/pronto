use std::cell::RefCell;

use objc2::{
    define_class, msg_send, rc::Retained, runtime::AnyObject, DefinedClass, MainThreadOnly,
};
use objc2_app_kit::{
    NSAccessibility, NSAccessibilityButtonRole, NSAccessibilityLayoutChangedNotification,
    NSAccessibilityPostNotification, NSButton, NSWindow,
};
use objc2_foundation::{NSArray, NSObjectProtocol, NSString};
use objc2_web_kit::WKWebView;
use serde::Deserialize;
use tauri::{Runtime, Webview};

const IDEAL_STATE_MANIFEST: &str = include_str!("../../.mac-control/ideal-state.json");

#[derive(Debug, Deserialize)]
struct Manifest {
    tasks: Vec<Task>,
}

#[derive(Debug, Deserialize)]
struct Task {
    accessibility: Accessibility,
}

#[derive(Debug, Deserialize)]
struct Accessibility {
    identifier: String,
    label: String,
}

fn evaluate_press(webview: &WKWebView, identifier: &NSString) {
    let script = NSString::from_str(&format!(
        "document.getElementById({})?.click()",
        serde_json::to_string(&identifier.to_string()).unwrap()
    ));
    unsafe {
        webview.evaluateJavaScript_completionHandler(&script, None);
    }
}

#[derive(Clone)]
struct ButtonIvars {
    webview: Retained<WKWebView>,
    identifier: Retained<NSString>,
    action_names: Retained<NSArray<NSString>>,
}

define_class!(
    #[unsafe(super(NSButton))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ButtonIvars]
    struct MacControlButton;

    impl MacControlButton {
        #[unsafe(method_id(initWithFrame:identifier:webView:))]
        fn init(
            this: objc2::rc::Allocated<Self>,
            frame: objc2_foundation::NSRect,
            identifier: &NSString,
            webview: &WKWebView,
        ) -> Retained<Self> {
            let this = this.set_ivars(ButtonIvars {
                webview: unsafe {
                    Retained::retain(webview as *const WKWebView as *mut WKWebView)
                        .expect("WKWebView must be retained")
                },
                identifier: unsafe {
                    Retained::retain(identifier as *const NSString as *mut NSString)
                        .expect("identifier must be retained")
                },
                action_names: NSArray::from_retained_slice(&[NSString::from_str("AXPress")]),
            });
            unsafe { msg_send![super(this), initWithFrame: frame] }
        }

        #[unsafe(method(accessibilityActionNames))]
        fn accessibility_action_names(&self) -> &NSArray<NSString> {
            self.ivars().action_names.as_ref()
        }

        #[unsafe(method(accessibilityPerformAction:))]
        fn accessibility_perform_action(&self, action: &NSString) {
            if action.to_string() == "AXPress" {
                evaluate_press(&self.ivars().webview, &self.ivars().identifier);
            }
        }

        #[unsafe(method(accessibilityPerformPress))]
        fn accessibility_perform_press(&self) -> bool {
            evaluate_press(&self.ivars().webview, &self.ivars().identifier);
            true
        }
    }

    unsafe impl NSObjectProtocol for MacControlButton {}
);

thread_local! {
    static BUTTONS: RefCell<Vec<Retained<MacControlButton>>> = const { RefCell::new(Vec::new()) };
}

fn manifest_targets() -> Result<Vec<Accessibility>, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str::<Manifest>(IDEAL_STATE_MANIFEST)?
        .tasks
        .into_iter()
        .map(|task| task.accessibility)
        .collect())
}

pub fn install<R: Runtime>(webview: Webview<R>) -> tauri::Result<()> {
    let targets = manifest_targets().expect("Pronto Mac Control manifest must parse");
    webview
        .with_webview(move |platform| unsafe {
            let mtm = objc2::MainThreadMarker::new().expect("Tauri webview is main-thread-only");
            let view: &WKWebView = &*platform.inner().cast();
            let window: &NSWindow = &*platform.ns_window().cast();
            let parent = window
                .contentView()
                .expect("Pronto window must have a native content view");

            BUTTONS.with(|buttons| {
                for button in buttons.borrow_mut().drain(..) {
                    button.removeFromSuperview();
                }
            });

            let mut buttons = Vec::with_capacity(targets.len());

            for target in targets {
                let identifier = NSString::from_str(&target.identifier);
                let label = NSString::from_str(&target.label);
                let frame = objc2_foundation::NSRect::new(
                    objc2_foundation::NSPoint::new(0.0, 0.0),
                    objc2_foundation::NSSize::new(1.0, 1.0),
                );
                let button: Retained<MacControlButton> = msg_send![
                    MacControlButton::alloc(mtm),
                    initWithFrame: frame,
                    identifier: &*identifier,
                    webView: view
                ];
                button.setTitle(&label);
                button.setAccessibilityIdentifier(Some(&identifier));
                button.setAccessibilityLabel(Some(&label));
                button.setAccessibilityRole(Some(NSAccessibilityButtonRole));
                button.setAccessibilityElement(true);
                button.setTransparent(true);
                button.setBordered(false);
                button.setEnabled(true);
                parent.addSubview(&button);
                buttons.push(button);
            }

            // Wry's parent view exposes an explicit accessibility child list. Keep the
            // web content visible there, and publish the proxy controls directly from the
            // window. Mac Control starts its bounded walk at AXWindow and Wry's wrapper
            // otherwise hides runtime AppKit subviews behind the WebKit subtree.
            let mut children: Vec<Retained<AnyObject>> = Vec::with_capacity(1);
            children.push(Retained::retain(view as *const WKWebView as *mut AnyObject).unwrap());
            let accessibility_children = NSArray::from_retained_slice(&children);
            parent.setAccessibilityElement(true);
            parent.setAccessibilityChildren(Some(&accessibility_children));

            let mut window_children: Vec<Retained<AnyObject>> =
                Vec::with_capacity(buttons.len() + 1);
            for button in &buttons {
                window_children.push(
                    Retained::retain(button.as_ref() as *const MacControlButton as *mut AnyObject)
                        .unwrap(),
                );
            }
            window_children.push(
                Retained::retain(parent.as_ref() as *const AnyObject as *mut AnyObject).unwrap(),
            );
            let window_accessibility_children = NSArray::from_retained_slice(&window_children);
            window.setAccessibilityElement(true);
            window.setAccessibilityChildren(Some(&window_accessibility_children));
            NSAccessibilityPostNotification(
                window.as_ref() as &AnyObject,
                NSAccessibilityLayoutChangedNotification,
            );

            BUTTONS.with(|stored| stored.borrow_mut().extend(buttons));
        })
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::manifest_targets;

    #[test]
    fn native_targets_are_derived_from_the_declared_manifest() {
        let targets = manifest_targets().expect("manifest must parse");
        assert_eq!(targets.len(), 4);
        assert_eq!(
            targets
                .iter()
                .map(|target| target.identifier.as_str())
                .collect::<Vec<_>>(),
            vec![
                "pronto.navigation.portfolio",
                "pronto.navigation.remediation",
                "pronto.remediation.refresh",
                "pronto.settings",
            ]
        );
        assert_eq!(
            targets
                .iter()
                .map(|target| target.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Portfolio", "Remediation", "Run full refresh", "Settings"]
        );
    }
}
