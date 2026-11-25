// The interface font, and one line of text in it.

import { type JSX, signal, Text } from "nsh";
import { type FontState, Status } from "nsvim-native";

export type { FontState };

/** The font until Rust gives the real one: config.toml's default size. */
const START: FontState = { family: "system-ui", size: 12.5, mono: "monospace" };

/** The interface font, as a signal that follows config.toml. Call it in a component body. */
export function useFont(): () => FontState {
  const font = signal<FontState>(START);
  Status.onFont((f) => font.set(f));
  return font;
}

/**
 * One line of text. The chrome has fixed heights, so its text follows config.toml's size and
 * not the system text size. `size` gives the size in points. Without it, the text is
 * `delta` points larger than the interface size (`delta` is 0 when left out). `mono` takes
 * the code font.
 */
export function Line(props: {
  font: () => FontState;
  size?: number;
  delta?: number;
  mono?: boolean;
  class?: string;
  testID?: string;
  children?: JSX.Element;
}) {
  return (
    <Text
      testID={props.testID}
      class={props.class}
      allowFontScaling={false}
      style={{
        numberOfLines: 1,
        fontFamily: props.mono ? props.font().mono : props.font().family,
        fontSize: props.size ?? Math.max(props.font().size + (props.delta ?? 0), 9),
      }}
    >
      {props.children}
    </Text>
  );
}
