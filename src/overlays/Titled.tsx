// A box with a 1 pt border and its title in the top edge, as the design draws panels.

import { platform, Text, View, type AccessibilityRole, type JSX } from "nsh";

/** The text size of the overlays' rows, titles and queries. */
export const SIZE = 12.5;

/** True in a terminal (`nsvim --tui`). A terminal lays views out in cells of 8 x 16 points,
 * so the overlays use sizes that are whole cells there: a box that ends in half a cell does
 * not cover what is under it. */
export const TERM = platform.idiom === "terminal";

/** A box with a border and `title` in its top edge. On GTK a view's border paints over its
 * children, so the border is an absolute child under the title. */
export function Titled(props: {
  title: string;
  class?: string;
  testID?: string;
  role?: AccessibilityRole;
  children?: JSX.Element;
}) {
  if (TERM)
    // In a terminal a box clips its children to inside its border, and a border on an empty
    // box draws nothing. So the box with the border holds the content, and the title is its
    // sibling over the top line, one cell in.
    return (
      <View class={props.class} testID={props.testID} role={props.role}>
        <View class="flex-1 min-h-0 w-full bg-background border border-border">{props.children}</View>
        <View class="absolute left-[16px] top-0 h-[16px] z-1 px-[8px] bg-background">
          <Text class="line-clamp-1 text-primary">{props.title}</Text>
        </View>
      </View>
    );
  return (
    <View class={`rounded-(--radius) bg-background ${props.class ?? ""}`} testID={props.testID} role={props.role}>
      <View class="absolute inset-0 rounded-(--radius) border border-border" />
      <View class="absolute left-[12px] top-[-9px] z-1 px-[5px] bg-background">
        <Text class="line-clamp-1 text-primary" style={{ fontSize: SIZE }}>
          {props.title}
        </Text>
      </View>
      {props.children}
    </View>
  );
}
