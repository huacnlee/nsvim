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

/** The group that holds `panel`. */
export function groupOf(layout: DockAreaLayout, panel: string): Tabs | undefined {
  return groups(layout.center).find((g) => g.panels.includes(panel));
}

/** `node` with its node `from` replaced by `to`. */
function replace(node: DockNode, from: DockNode, to: DockNode): DockNode {
  if (node === from) return to;
  return node.type === "split" ? { ...node, children: node.children.map((c) => replace(c, from, to)) } : node;
}

/** The split that holds `child` directly. */
function parentOf(node: DockNode, child: DockNode): Extract<DockNode, { type: "split" }> | undefined {
  if (node.type !== "split") return undefined;
  if (node.children.includes(child)) return node;
  for (const c of node.children) {
    const p = parentOf(c, child);
    if (p) return p;
  }
  return undefined;
}

/** `layout` with `panel` moved out of its group to a new group after it: on the right for
 * `"horizontal"`, below for `"vertical"`. In a split of the same axis the new group is the
 * next child, and the children share the space again. `undefined` when the group has no other
 * panel to keep, as DockArea keeps each panel in one group. */
export function split(layout: DockAreaLayout, panel: string, axis: "horizontal" | "vertical"): DockAreaLayout | undefined {
  const g = groupOf(layout, panel);
  if (!g || g.panels.length < 2) return undefined;
  const panels = g.panels.filter((p) => p !== panel);
  const shown = g.panels[g.activeIndex ?? 0];
  const at = shown === panel ? Math.min(g.panels.indexOf(panel), panels.length - 1) : panels.indexOf(shown);
  const kept: Tabs = { ...g, panels, activeIndex: Math.max(0, at) };
  const fresh: Tabs = { type: "tabs", panels: [panel], activeIndex: 0 };
  const parent = parentOf(layout.center, g);
  let center: DockNode;
  if (parent && parent.axis === axis) {
    const i = parent.children.indexOf(g);
    const children = [...parent.children.slice(0, i), kept, fresh, ...parent.children.slice(i + 1)];
    const node = { ...parent, children, sizes: children.map(() => null) };
    center = replace(layout.center, parent, node);
  } else {
    center = replace(layout.center, g, { type: "split", axis, children: [kept, fresh], sizes: [null, null] });
  }
  return { ...layout, center };
}

/** The panel that the group `step` groups away from the group of `panel` displays, in tree
 * order, round from the end to the start. */
export function neighbour(layout: DockAreaLayout, panel: string, step: number): string | undefined {
  const all = groups(layout.center).filter((g) => g.panels.length > 0);
  const at = all.findIndex((g) => g.panels.includes(panel));
  if (at < 0 || all.length < 2) return undefined;
  const g = all[(at + step + all.length) % all.length];
  return g.panels[g.activeIndex ?? 0];
}

/** `layout` without the group of `panel`: its panels join the group before it (the first group:
 * the one after it), after that group's panels, and `panel` shows there. `undefined` with one
 * group. */
export function closeGroup(layout: DockAreaLayout, panel: string): DockAreaLayout | undefined {
  const all = groups(layout.center).filter((g) => g.panels.length > 0);
  const at = all.findIndex((g) => g.panels.includes(panel));
  if (at < 0 || all.length < 2) return undefined;
  const g = all[at];
  const into = all[at > 0 ? at - 1 : 1];
  const panels = [...into.panels, ...g.panels];
  const center = replace(replace(layout.center, g, { ...g, panels: [] }), into, { ...into, panels, activeIndex: panels.indexOf(panel) });
  return { ...layout, center: dropEmpty(center) ?? { type: "tabs", panels } };
}

/** `node` without empty groups, and a split with one child left is that child. */
function dropEmpty(node: DockNode): DockNode | undefined {
  if (node.type === "tabs") return node.panels.length > 0 ? node : undefined;
  const kids: DockNode[] = [];
  const sizes: (number | null)[] = [];
  node.children.forEach((c, i) => {
    const k = dropEmpty(c);
    if (k) {
      kids.push(k);
      sizes.push(node.sizes?.[i] ?? null);
    }
  });
  if (kids.length === 0) return undefined;
  if (kids.length === 1) return kids[0];
  return { ...node, children: kids, sizes: kids.length === node.children.length ? node.sizes : kids.map(() => null) };
}
