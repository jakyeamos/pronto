import { useEffect, type RefObject } from "react";

export function useAppKeyboardShortcuts(
  searchInputRef: RefObject<HTMLInputElement | null>,
  onEscape: () => void,
): void {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent): void => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        searchInputRef.current?.focus();
      }
      if (event.key === "Escape") onEscape();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onEscape, searchInputRef]);
}
