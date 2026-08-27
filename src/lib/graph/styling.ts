// Ported from studio_legacy/components/graph/styling.rs

import type { NodeKind } from "./types";

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
      return "bg-green-900";
    case "input":
      return "bg-blue-900";
    case "output":
      return "bg-red-900";
  }
}
