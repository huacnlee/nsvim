// The first three steps of setup: keys, mouse and look. A press picks a choice through
// `SetupFlow.choose`, as Enter does.

import { Text, View } from "nsh";
import { RadioGroup } from "nsh/base";
import { type SetupLook, type SetupState } from "nsvim-native";
import { SetupFlow } from "nsvim-native";
import { Choice, ChoiceGroup, Pill, Titled } from "../controls";
import { Card, Heading, KeyRow, OptionBox, CheckRow } from "./parts";

type Step = { st: () => SetupState };

const choose = (id: string) => () => SetupFlow.choose(id);

export function KeysStep(props: Step) {
  const st = props.st;
  const md = () => (st().mac ? "⌘" : "Ctrl");
  const leader = () => (st().on.includes("leader") ? "SPC" : "\\");
  const platform = (mac: boolean, label: string) => {
    const id = mac ? "platform:mac" : "platform:other";
    return <Choice id={`platform:${label}`} label={label} first={mac} on={st().mac === mac} focus={st().focus === id} onPress={choose(id)} />;
  };
  return (
    <View class="gap-5.5" testID="setup-step-1">
      <View class="flex-row items-end flex-wrap gap-5">
        <Heading title="How do you want to edit?" sub="Switch later with :set keymap." />
        <View class="flex-1" />
        <View class="flex-row gap-2 items-center">
          <Text class="text-xs text-muted-foreground">Keys shown for</Text>
          <ChoiceGroup label="Keys shown for">
            {platform(true, "macOS")}
            {platform(false, "Linux / Windows")}
          </ChoiceGroup>
        </View>
      </View>
      <RadioGroup accessibilityLabel="Keymap" class="flex-row gap-4 flex-wrap items-start">
        <Card
          st={st}
          choice="keymap:vim"
          tag="neovim"
          title="Neovim motions"
          desc="Modal: move in Normal, type in Insert. Your muscle memory works as is."
          on={st().vim}
        >
          <View class="gap-3">
            <View class="flex-row gap-1.5">
              <Pill fill="bg-primary" ink="text-primary-foreground">
                NORMAL
              </Pill>
              <Pill fill="bg-mode-insert" ink="text-primary-foreground">
                INSERT
              </Pill>
              <Pill fill="bg-mode-visual" ink="text-primary-foreground">
                VISUAL
              </Pill>
            </View>
            <View>
              <KeyRow name="Save" keys=":w" />
              <KeyRow name="Find file" keys={`${leader()} f`} />
              <KeyRow name="Search" keys="/" />
              <KeyRow name="Comment" keys="gcc" />
            </View>
          </View>
        </Card>
        <Card
          st={st}
          choice="keymap:classic"
          tag="classic"
          title="Classic shortcuts"
          desc="The keys from most editors. No modes. Start typing straight away."
          on={!st().vim}
        >
          <View class="gap-3">
            <View class="flex-row gap-1.5">
              <Pill fill="bg-surface-raised" ink="text-foreground">
                {md()}
              </Pill>
              <Pill fill="bg-surface-raised" ink="text-foreground">
                {st().mac ? "⌥" : "Alt"}
              </Pill>
              <Pill fill="bg-surface-raised" ink="text-foreground">
                ⇧
              </Pill>
            </View>
            <View>
              <KeyRow name="Save" keys={`${md()} S`} />
              <KeyRow name="Find file" keys={`${md()} P`} />
              <KeyRow name="Search" keys={`${md()} F`} />
              <KeyRow name="Comment" keys={`${md()} /`} />
            </View>
          </View>
        </Card>
      </RadioGroup>
      <OptionBox title="Options">
        <If
          when={st().vim}
          else={
            <>
              <CheckRow st={st} choice="toggle:multi" label="Add cursors with Alt-click" sub="Every cursor types at once." />
              <CheckRow st={st} choice="toggle:autoclose" label="Auto-close brackets and quotes" sub="Typing ( also types )." />
            </>
          }
        >
          <CheckRow st={st} choice="toggle:leader" label="Use Space as the leader key" sub="Otherwise backslash, as in stock Neovim." />
          <CheckRow st={st} choice="toggle:import" label="Import keymaps from ~/.config/nvim" sub="Reads your leader. Plugins are skipped." />
        </If>
      </OptionBox>
    </View>
  );
}

export function MouseStep(props: Step) {
  const st = props.st;
  const vim = () => st().vim;
  return (
    <View class="gap-5.5" testID="setup-step-2">
      <Heading title="Use the mouse?" sub="nsvim is keyboard-first. This only changes the text area and panes." />
      <RadioGroup accessibilityLabel="Mouse" class="flex-row gap-4 flex-wrap items-start">
        <Card
          st={st}
          choice="mouse:on"
          tag="mouse=a"
          title="Mouse on"
          desc="Click to place the cursor, drag to select, scroll, resize the file tree."
          on={st().mouse}
        >
          <View class="rounded-(--radius) border border-border p-2">
            <Text class="text-xs text-foreground">{"let mut pos = from;\nfor _ in 0..count.max(1) {\n    pos = buf.next(pos);"}</Text>
          </View>
        </Card>
        <Card
          st={st}
          choice="mouse:off"
          tag="mouse="
          title="Keyboard only"
          desc="Clicks inside the editor do nothing, so a stray click never moves the cursor. The wheel still scrolls."
          on={!st().mouse}
        >
          <View>
            <KeyRow name="move" keys={vim() ? "h j k l" : "← ↑ ↓ →"} />
            <KeyRow name="select" keys={vim() ? "v" : "⇧ arrows"} />
          </View>
        </Card>
      </RadioGroup>
      <If when={st().mouse}>
        <OptionBox title="The mouse can">
          <CheckRow st={st} choice="toggle:click" label="Click places the cursor" />
          <If when={vim()} else={<CheckRow st={st} choice="toggle:drag" label="Drag selects text" />}>
            <CheckRow st={st} choice="toggle:drag" label="Drag selects and enters Visual mode" />
          </If>
          <CheckRow st={st} choice="toggle:double" label="Double-click selects a word, triple-click a line" />
          <CheckRow st={st} choice="toggle:right" label="Right-click opens a context menu" />
        </OptionBox>
      </If>
    </View>
  );
}

const TITLES: Record<string, string> = { light: "Light", dark: "Dark", system: "System" };

export function LookStep(props: Step) {
  const st = props.st;
  const desc = (id: string) => {
    if (id === "light") return "Paper white, dark ink.";
    if (id === "dark") return "The default. Low glare at night.";
    // On Omarchy, System follows the desktop theme. Elsewhere it follows the OS.
    const name = st().omarchy;
    return name ? `Follows the Omarchy theme, now ${name}.` : "Light or dark with your OS.";
  };
  const font = (text: string, ink: string) => (
    <Text class={ink} style={{ fontFamily: st().fontFamily, fontSize: st().fontSize }}>
      {text}
    </Text>
  );
  return (
    <View class="gap-5.5" testID="setup-step-3">
      <Heading title="Pick a look" sub="This window follows your pick. Colors, fonts and spacing are all in Settings later." />
      {/* The cards in a row have the same height, as tall as the tallest. */}
      <RadioGroup accessibilityLabel="Look" class="flex-row gap-3 flex-wrap items-stretch">
        <Each of={st().looks} key={(l) => l.id}>
          {(look, i) => (
            <Card
              st={st}
              choice={`look:${i()}`}
              tag={look().id}
              title={TITLES[look().id] ?? look().id}
              desc={desc(look().id)}
              on={st().themeMode === look().id}
              class="min-w-47.5"
            >
              <Sample look={look} />
            </Card>
          )}
        </Each>
      </RadioGroup>
      <Titled title="Font" class="w-full" bodyClass="pt-4 pb-3 px-4 gap-3">
        <View class="flex-row gap-4 items-center">
          <Label>Family</Label>
          <ChoiceGroup label="Font family">
            <Each of={st().fonts}>
              {(f, i) => (
                <Choice
                  id={`font:${f()}`}
                  label={f()}
                  first={i() === 0}
                  on={st().font === f()}
                  focus={st().focus === `font:${i()}`}
                  onPress={() => SetupFlow.choose(`font:${i()}`)}
                />
              )}
            </Each>
          </ChoiceGroup>
        </View>
        <View class="flex-row gap-4 items-center">
          <Label>Size</Label>
          <ChoiceGroup label="Font size">
            <Each of={st().sizes}>
              {(z, i) => (
                <Choice
                  id={`size:${z()}`}
                  label={`${z()} pt`}
                  first={i() === 0}
                  on={st().fontSize === z()}
                  focus={st().focus === `size:${i()}`}
                  onPress={() => SetupFlow.choose(`size:${i()}`)}
                />
              )}
            </Each>
          </ChoiceGroup>
        </View>
        <View class="rounded-(--radius) p-2 border border-border">
          {font("/// The quick brown fox -> jumps != over", "text-code-comment")}
          {font("pub fn word_forward(buf: &Buffer, from: Pos) -> Target {", "text-foreground")}
        </View>
      </Titled>
    </View>
  );
}

function Label(props: { children: string }) {
  return <Text class="w-16 text-xs text-muted-foreground">{props.children}</Text>;
}

/** A few lines of Rust in a look's own colours. The colours are data from Rust: the look is not
 * the theme that shows now, so no token has them. */
function Sample(props: { look: () => SetupLook }) {
  return (
    <View class="rounded-(--radius) p-2 border" style={{ backgroundColor: props.look().background, borderColor: props.look().border }}>
      <Each of={props.look().lines} key="index">
        {(line) => (
          <Text class="text-xs line-clamp-1">
            <Each of={line().spans} key="index">
              {(s) => <Text style={{ color: s().color }}>{s().text}</Text>}
            </Each>
          </Text>
        )}
      </Each>
    </View>
  );
}
