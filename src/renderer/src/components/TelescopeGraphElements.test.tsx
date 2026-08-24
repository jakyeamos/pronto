// @vitest-environment happy-dom

import { fireEvent, render, screen } from "@testing-library/react";
import type { EdgeProps } from "@xyflow/react";
import { describe, expect, it, vi } from "vitest";
import { TelescopeFlowEdge } from "./TelescopeGraphElements";

function edgeProps(overrides: Partial<EdgeProps> = {}): EdgeProps {
  return {
    id: "edge",
    source: "source",
    target: "target",
    sourceX: 0,
    sourceY: 0,
    targetX: 120,
    targetY: 80,
    sourcePosition: "right",
    targetPosition: "left",
    data: {},
    ...overrides,
  } as EdgeProps;
}

describe("TelescopeFlowEdge", () => {
  it("keeps a reduced-motion flow token static, inspectable, and keyboard accessible", () => {
    const onSelectToken = vi.fn();
    render(
      <svg>
        <TelescopeFlowEdge
          {...edgeProps({
            data: {
              activeToken: true,
              reducedMotion: true,
              onSelectToken,
            },
          })}
        />
      </svg>,
    );

    const token = screen.getByRole("button", {
      name: "Inspect static flow token",
    });
    expect(token.querySelector("animateMotion")).toBeNull();
    fireEvent.keyDown(token, { key: "Enter" });
    expect(onSelectToken).toHaveBeenCalledTimes(1);
  });
});
