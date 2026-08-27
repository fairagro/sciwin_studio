// Ported from studio_legacy/components/graph/styling.rs

import type { NodeKind, PickValue } from "./types";

const BG_BY_TYPE: Record<string, string> = {
  file: "bg-green-400",
  directory: "bg-blue-400",
  string: "bg-red-400",
  boolean: "bg-yellow-400",
  double: "bg-purple-400",
  float: "bg-pink-400",
  long: "bg-cyan-400",
  int: "bg-teal-400",
};

function parts(dataType: string): string[] {
  return dataType.split("|").map((p) => p.trim().toLowerCase());
}

function primaryType(dataType: string): string {
  return parts(dataType).find((p) => p !== "null") ?? "null";
}

function isOptional(dataType: string): boolean {
  return parts(dataType).includes("null");
}

function isArrayLike(dataType: string): boolean {
  return primaryType(dataType) === "array";
}

export function portBg(dataType: string): string {
  if (dataType === "stdout") return "bg-slate-300";
  if (dataType === "stderr") return "bg-slate-200";
  return BG_BY_TYPE[primaryType(dataType)] ?? "bg-slate-400";
}

export function portGeometry(dataType: string): string {
  const t = primaryType(dataType);
  return t === "file" || t === "directory" || dataType === "stdout" || dataType === "stderr"
    ? "rotate-45"
    : "rounded-full";
}

export function portBorder(dataType: string): string {
  if (isArrayLike(dataType)) return "border border-green-800";
  if (isOptional(dataType)) return "border border-red-800";
  return "border border-black";
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
