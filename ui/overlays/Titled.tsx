// A box with a 1 pt border and its title in the top edge, as the design draws panels.

import { Text, View, type AccessibilityRole, type JSX } from "nsh";

/** The text size of the overlays' rows, titles and queries. */
export const SIZE = 12.5;

/** A box with a border and `title` in its top edge. On GTK a view's border paints over its
 * children, so the border is an absolute child under the title. */
export function Titled(props: {
  title: string;
  class?: string;
  testID?: string;
  role?: AccessibilityRole;
  children?: JSX.Element;
}) {
  return (
    <View class={`rounded-[6px] bg-background ${props.class ?? ""}`} testID={props.testID} role={props.role}>
      <View class="absolute inset-0 rounded-[6px] border border-border" />
      <View class="absolute left-[12px] top-[-9px] z-1 px-[5px] bg-background">
        <Text class="line-clamp-1 text-primary" style={{ fontSize: SIZE }}>
          {props.title}
        </Text>
      </View>
      {props.children}
    </View>
  );
}
