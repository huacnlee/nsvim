// The live preview of Settings: a short Rust file drawn with the current font, line numbers,
// cursor and whitespace options, and the config.toml lines that the options write. Rust finds
// the colour of each run (`SettingsPage.onPreview`). Here each run gets its token class.

import { signal, Text, View } from "nsh";
import { type PreviewSpan, type SettingsPreview, SettingsPage } from "nsvim-native";
import { Titled } from "../controls";

/** The text class of each colour token that the preview uses. */
const INK: Record<string, string> = {
  foreground: "text-foreground",
  border: "text-border",
  "code-keyword": "text-code-keyword",
  "code-function": "text-code-function",
  "code-type": "text-code-type",
  "code-string": "text-code-string",
  "code-number": "text-code-number",
  "code-comment": "text-code-comment",
  "code-macro": "text-code-macro",
  "code-property": "text-code-property",
  "code-punctuation": "text-code-punctuation",
};

/** The classes of the char under each cursor shape. A block fills it, a bar and an underline
 * mark it. */
const CURSOR: Record<string, string> = {
  block: "bg-foreground text-background",
  bar: "text-primary bg-surface-raised",
  underline: "underline text-foreground",
};

const EMPTY: SettingsPreview = { family: "", size: 13, rowHeight: 20, lines: [], toml: [] };

export function Preview() {
  const p = signal<SettingsPreview>(EMPTY);
  SettingsPage.onPreview((v) => p.set(v));
  const mono = () => ({ fontFamily: p().family || undefined, fontSize: p().size });
  return (
    <View class="w-120 shrink-0 pt-4.5 pr-5 gap-4.5" testID="settings-preview">
      <Titled title="Preview · motion.rs" class="w-full">
        <View class="w-full pt-3.5 pb-3 overflow-hidden">
          <Each of={p().lines} key="index">
            {(line) => (
              <View
                class="flex-row items-center"
                classList={{ "bg-surface-raised": line().highlight, "bg-background": !line().highlight }}
                style={{ height: p().rowHeight }}
              >
                <View class="items-end" style={{ width: p().size * 3, marginRight: p().size * 1.2 }}>
                  <Text
                    allowFontScaling={false}
                    class="line-clamp-1"
                    style={mono()}
                    classList={{ "text-foreground": line().current, "text-text-faint": !line().current }}
                  >
                    {line().number}
                  </Text>
                </View>
                <Text allowFontScaling={false} class="line-clamp-1" style={mono()}>
                  <Each of={line().spans} key="index">
                    {(s) => <Span span={s} />}
                  </Each>
                </Text>
              </View>
            )}
          </Each>
        </View>
      </Titled>
      <Titled title="config.toml" class="w-full">
        <View class="w-full pt-4 pb-3 px-4 gap-0.5" testID="settings-toml">
          <Each of={p().toml} key="index">
            {(t) => (
              <Text class="line-clamp-1" allowFontScaling={false} style={{ fontFamily: p().family || undefined, fontSize: 12 }}>
                <Text class="text-code-property">{t().key}</Text>
                <Text class="text-code-punctuation"> = </Text>
                <Text class={INK[t().ink] ?? "text-foreground"}>{t().value}</Text>
              </Text>
            )}
          </Each>
        </View>
      </Titled>
    </View>
  );
}

/** One run of code: its token colour, italic for a comment, and the cursor's look. */
function Span(props: { span: () => PreviewSpan }) {
  const cls = () => {
    const s = props.span();
    const look = s.cursor ? CURSOR[s.cursor] : (INK[s.ink] ?? "text-foreground");
    return s.italic ? `${look} italic` : look;
  };
  return <Text class={cls()}>{props.span().text}</Text>;
}
