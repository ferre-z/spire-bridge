import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { GlassButton } from "@/components/glass/GlassButton";
import { GlassPanel } from "@/components/glass/GlassPanel";
import { GlassPill } from "@/components/glass/GlassPill";
import { GlassCard } from "@/components/glass/GlassCard";
import { Icon } from "@/components/primitives/Icon";

describe("GlassPill", () => {
  it("applies the accent tone background class for tone='accent'", () => {
    render(<GlassPill tone="accent">Live</GlassPill>);
    const pill = screen.getByText("Live");
    expect(pill.className).toContain("bg-red-500/20");
    expect(pill.className).toContain("rounded-full");
  });

  it("applies the neutral tone by default", () => {
    render(<GlassPill>Idle</GlassPill>);
    const pill = screen.getByText("Idle");
    expect(pill.className).toContain("bg-white/[0.06]");
  });

  it("supports every declared tone without falling through", () => {
    const tones = [
      "neutral",
      "success",
      "warning",
      "error",
      "info",
      "accent",
    ] as const;
    for (const tone of tones) {
      const { unmount } = render(
        <GlassPill tone={tone} data-testid={`pill-${tone}`}>
          {tone}
        </GlassPill>,
      );
      const el = screen.getByTestId(`pill-${tone}`);
      expect(el.className).not.toContain("undefined");
      unmount();
    }
  });
});

describe("GlassPanel", () => {
  it("has the glass base class", () => {
    render(<GlassPanel data-testid="panel" />);
    expect(screen.getByTestId("panel").className).toContain("glass");
  });

  it("adds the glass-hover class when interactive", () => {
    render(<GlassPanel interactive data-testid="panel" />);
    const panel = screen.getByTestId("panel");
    expect(panel.className).toContain("glass");
    expect(panel.className).toContain("glass-hover");
    expect(panel.className).toContain("cursor-pointer");
  });

  it("adds the glass-strong class when strong", () => {
    render(<GlassPanel strong data-testid="panel" />);
    expect(screen.getByTestId("panel").className).toContain("glass-strong");
  });
});

describe("GlassCard", () => {
  it("applies default md padding", () => {
    render(<GlassCard data-testid="card">body</GlassCard>);
    expect(screen.getByTestId("card").className).toContain("p-5");
  });

  it("respects size='lg'", () => {
    render(
      <GlassCard size="lg" data-testid="card">
        body
      </GlassCard>,
    );
    expect(screen.getByTestId("card").className).toContain("p-6");
  });
});

describe("GlassButton", () => {
  it("calls onClick when clicked", () => {
    const onClick = vi.fn();
    render(
      <GlassButton onClick={onClick} data-testid="btn">
        Go
      </GlassButton>,
    );
    fireEvent.click(screen.getByTestId("btn"));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("renders the primary variant class", () => {
    render(
      <GlassButton variant="primary" data-testid="btn">
        Go
      </GlassButton>,
    );
    const btn = screen.getByTestId("btn");
    expect(btn.className).toContain("bg-[#ef4444]");
  });

  it("does not call onClick when disabled", () => {
    const onClick = vi.fn();
    render(
      <GlassButton disabled onClick={onClick} data-testid="btn">
        Go
      </GlassButton>,
    );
    fireEvent.click(screen.getByTestId("btn"));
    expect(onClick).not.toHaveBeenCalled();
  });

  it("supports fullWidth", () => {
    render(
      <GlassButton fullWidth data-testid="btn">
        Stretch
      </GlassButton>,
    );
    expect(screen.getByTestId("btn").className).toContain("w-full");
  });
});

describe("Icon", () => {
  it("renders a lucide icon by name", () => {
    const { container } = render(
      <Icon name="Zap" data-testid="icon" aria-label="zap" />,
    );
    // lucide-react renders an <svg>; check it landed in the tree.
    expect(container.querySelector("svg")).toBeTruthy();
  });
});
