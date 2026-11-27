// The layout of the panes: the open files as the panels of an NSH `DockArea`, kept in step with
// the tabs that Rust lists. Each panel's key is its document's id (`TabState.id`). Plain
// functions on the layout value, with no view code.

import type { DockAreaLayout } from "nsh/base";

type DockNode = DockAreaLayout["center"];
type Tabs = Extract<DockNode, { type: "tabs" }>;

/** The tab groups of `node`, in tree order: left to right, top to bottom. */
export function groups(node: DockNode): Tabs[] {
  return node.type === "tabs" ? [node] : node.children.flatMap(groups);
}

/** The panels of `layout`, in tree order. */
export function panelsOf(layout: DockAreaLayout): string[] {
  return groups(layout.center).flatMap((g) => g.panels);
}

/** `node` without the panels that `keep` does not hold. A group left empty stays, and the area
 * removes it when it takes the layout. */
function prune(node: DockNode, keep: Set<string>): DockNode {
  if (node.type === "split") return { ...node, children: node.children.map((c) => prune(c, keep)) };
  const panels = node.panels.filter((p) => keep.has(p));
  const shown = node.panels[node.activeIndex ?? 0];
  const at = panels.indexOf(shown);
  return { ...node, panels, activeIndex: at >= 0 ? at : Math.max(0, Math.min(node.activeIndex ?? 0, panels.length - 1)) };
}

/** `node` with `f` applied to each tab group. */
function mapGroups(node: DockNode, f: (g: Tabs) => Tabs): DockNode {
  return node.type === "tabs" ? f(node) : { ...node, children: node.children.map((c) => mapGroups(c, f)) };
}

/** The layout for the tabs `keys` (in Rust's order) with `active` the file that has the keys.
 * `near` is the panel that had the keys before: a new file opens in its group, after it.
 * Files that closed leave their groups. The group of the active file displays it. */
export function sync(layout: DockAreaLayout, keys: string[], active: string | undefined, near: string | undefined): DockAreaLayout {
  const keep = new Set(keys);
  let center = prune(layout.center, keep);
  const placed = new Set(groups(center).flatMap((g) => g.panels));
  const fresh = keys.filter((k) => !placed.has(k));
  if (fresh.length > 0) {
    const all = groups(center);
    let target = all.find((g) => near !== undefined && g.panels.includes(near)) ?? all.find((g) => g.panels.length > 0) ?? all[0];
    if (!target) {
      center = { type: "tabs", panels: [] };
      target = center;
    }
    const goal = target;
    center = mapGroups(center, (g) => {
      if (g !== goal) return g;
      const at = near !== undefined ? g.panels.indexOf(near) : -1;
      const panels = at >= 0 ? [...g.panels.slice(0, at + 1), ...fresh, ...g.panels.slice(at + 1)] : [...g.panels, ...fresh];
      return { ...g, panels };
    });
  }
  if (active !== undefined) {
    center = mapGroups(center, (g) => {
      const at = g.panels.indexOf(active);
      return at >= 0 && at !== (g.activeIndex ?? 0) ? { ...g, activeIndex: at } : g;
    });
  }
  return { ...layout, center };
}

/** The same panels in the same groups, the same displayed tabs and the same splits. */
export function sameLayout(a: DockAreaLayout, b: DockAreaLayout): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}
