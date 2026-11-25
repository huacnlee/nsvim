// The picked file's start, or the lines around the picked line, coloured as code. Rust
// colours the lines. This view only draws them.

import { signal, Text, View } from "nsh";
import { Overlays, type PreviewState } from "nsvim-native";

import { SIZE, Titled } from "./Titled";

/** The text class of each token that Rust names. The classes are static, so the build checks
 * them. */
const TOKEN: Record<string, string> = {
  "code-keyword": "text-code-keyword",
  "code-function": "text-code-function",
  "code-type": "text-code-type",
  "code-string": "text-code-string",
  "code-number": "text-code-number",
  "code-comment": "text-code-comment",
  "code-macro": "text-code-macro",
  "code-property": "text-code-property",
  "code-punctuation": "text-code-punctuation",
  foreground: "text-foreground",
};

export function Preview() {
  const part = signal<PreviewState>({ title: "", font: "", lines: [] });
  Overlays.onPreview((p) => part.set(p));
  return (
    <Titled title={part().title} class="flex-1 min-w-0 h-full" testID="picker-preview">
      <View class="flex-1 min-h-0 pt-[12px] overflow-hidden">
        <Each of={part().lines} key="index">
          {(line) => (
            <View class="flex-row h-[21px] items-center" classList={{ "bg-selection": line().hit }}>
              <Text
                class="w-[44px] pr-[14px] text-right text-text-faint"
                style={{ fontSize: SIZE, fontFamily: part().font }}
              >
                {String(line().number)}
              </Text>
              <Text class="line-clamp-1" style={{ fontSize: SIZE, fontFamily: part().font }}>
                <Each of={line().runs} key="index">
                  {(run) => <Text class={TOKEN[run().token] ?? "text-foreground"}>{run().text}</Text>}
                </Each>
              </Text>
            </View>
          )}
        </Each>
      </View>
    </Titled>
  );
}
