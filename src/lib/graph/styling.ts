// Ported from studio_legacy/components/graph/styling.rs

import type { NodeKind, PickValue } from "./types";

const BG_BY_TYPE: Record<string, string> = {
  file: "bg-green-400!",
  directory: "bg-blue-400!",
  string: "bg-red-400!",
  boolean: "bg-yellow-400!",
  double: "bg-purple-400!",
  float: "bg-pink-400!",
  long: "bg-cyan-400!",
  int: "bg-teal-400!",
};

function parts(dataType: string): string[] {
  return dataType.split("|").map((p) => p.trim().toLowerCase());
}

// e.g. "File[]" for an array, plain otherwise. Only the outer "| null" split
// is handled here -- a multi-item array like "File | string[]" (rare in
// practice) isn't parsed apart correctly, same limitation the old "array"
// label had, just one level deeper.
function nonNullPart(dataType: string): string {
  return parts(dataType).find((p) => p !== "null") ?? "null";
}

function isOptional(dataType: string): boolean {
  return parts(dataType).includes("null");
}

function isArrayLike(dataType: string): boolean {
  return nonNullPart(dataType).endsWith("[]");
}

// The item type an array port is colored by, e.g. "file" for "File[]" --
// falls back to the bare type for anything that isn't an array.
function primaryType(dataType: string): string {
  const part = nonNullPart(dataType);
  return isArrayLike(dataType) ? part.slice(0, -2) : part;
}

function baseBg(dataType: string): string {
  if (dataType === "stdout") return "bg-slate-300!";
  if (dataType === "stderr") return "bg-slate-200!";
  return BG_BY_TYPE[primaryType(dataType)] ?? "bg-slate-400!";
}

// Solid fill = required; hollow (node-surface color instead of the type
// color, type-colored border instead) = optional. Replaces a colored ring,
// which collided with the type-color fill itself -- a red `string?` port
// got a red ring on a red dot, unreadable regardless of which red it was.
export function portBg(dataType: string): string {
  return isOptional(dataType) ? "bg-bg-surface!" : baseBg(dataType);
}

// Same color as the port itself, as a `stroke` value instead of a `bg-*`
// class -- edges are SVG, not DOM elements Tailwind can target. Reusing
// baseBg() (not portBg()) so an edge out of an optional port still carries
// its real type color rather than the hollow-port "no fill" one; slice off
// both the "bg-" prefix and the trailing "!" it always carries.
export function edgeStrokeColor(dataType: string): string {
  return `var(--color-${baseBg(dataType).slice(3, -1)})`;
}

export function portGeometry(dataType: string): string {
  const t = primaryType(dataType);
  return t === "file" || t === "directory" || dataType === "stdout" || dataType === "stderr"
    ? "rotate-45!"
    : "rounded-full!";
}

const BORDER_BY_TYPE: Record<string, string> = {
  file: "border-green-400!",
  directory: "border-blue-400!",
  string: "border-red-400!",
  boolean: "border-yellow-400!",
  double: "border-purple-400!",
  float: "border-pink-400!",
  long: "border-cyan-400!",
  int: "border-teal-400!",
};

function baseBorderColor(dataType: string): string {
  if (dataType === "stdout") return "border-slate-300!";
  if (dataType === "stderr") return "border-slate-200!";
  return BORDER_BY_TYPE[primaryType(dataType)] ?? "border-slate-400!";
}

// A hollow (optional) port needs its border to carry the type color, since
// the fill no longer does. A solid (required) port keeps the original
// plain, low-key outline -- the "punched through the node" look, not a
// second color signal.
export function portBorder(dataType: string): string {
  return isOptional(dataType) ? `border-2! ${baseBorderColor(dataType)}` : "border! border-black!";
}

// Array gets its own ring, independent of portBorder (already spoken for by
// required/optional) so the two compose instead of colliding on the same
// property -- `outline` doesn't affect layout and isn't touched by
// .svelte-flow__handle, so no `!important` fight to win here.
export function portArrayRing(dataType: string): string {
  return isArrayLike(dataType) ? "outline-2 outline-white outline-offset-1" : "";
}

export function nodeHeaderClass(kind: NodeKind): string {
  switch (kind) {
    case "step":
      return "bg-green-hdr";
    case "input":
      return "bg-blue-hdr";
    case "output":
      return "bg-red-hdr";
  }
}

// Halo ring matching the node body -- the ComfyUI-style "punched through the
// border" look for port dots. box-shadow, not border, so it layers with
// portBorder()'s semantic array/optional ring instead of overwriting it.
export const portRing = "shadow-[0_0_0_2px_var(--color-bg-surface)]";

// pickValue only exists on a multi-source input, so it's shown on the edges
// converging on that port, not on the node -- unlike scatter/when, which are
// step-wide.
export const pickValueEdgeStyle = "stroke-dasharray: 4 3;";
export const pickValueLabelStyle =
  "background: var(--color-bg-panel); border: 1px solid var(--color-border); border-radius: 4px; padding: 1px 6px; font-size: 10px; font-family: 'IBM Plex Mono', monospace; color: var(--color-text-2); white-space: nowrap;";

const PICK_VALUE_LABEL: Record<PickValue, string> = {
  first_non_null: "first non-null",
  the_only_non_null: "only non-null",
  all_non_null: "all non-null",
};

export function pickValueLabel(value: PickValue): string {
  return PICK_VALUE_LABEL[value];
}
