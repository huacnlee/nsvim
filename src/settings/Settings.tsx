// Settings: a layer over the whole window. A header with the title and a close button is on
// top. Under it are the sections on the left, the options of one section in the middle and, on
// a wide window, a live preview and the config.toml lines on the right. A change applies at once
// and goes to config.toml at once. Rust owns the state (`SettingsPage`) and the keys: Esc and
// Ctrl+W (⌘W on macOS) close it. A press outside a control does nothing.

import { signal, Text, View, Scroll } from "nsh";
import { Button } from "nsh/base";
import { Icon, type SettingsItem, type SettingsState, SettingsPage, Shell } from "nsvim-native";
import { act, Check, Chips, Choice, ChoiceGroup, FontSelect, Stepper } from "../controls";
import { Preview } from "./Preview";
import { Controls, controlsLeft, DRAG } from "../title-bar";

/** Wider than this, the page shows the preview. */
const PREVIEW_FROM = 1100;

const EMPTY: SettingsState = { sections: [], section: "", row: 0, items: [], configPath: "" };

/** Settings, over the whole window. */
export function Settings() {
  const state = signal<SettingsState>(EMPTY);
  SettingsPage.onChange((s) => state.set(s));
  const width = signal(0);
  return (
    // A press outside a control only gives the keys back. It never closes Settings.
    <View
      class="absolute inset-0 z-50 bg-background"
      testID="settings"
      onPress={() => Shell.focusKeys()}
      onLayout={(e) => width.set(e.bounds.size.width)}
    >
      <Header state={state} />
      <View class="flex-row flex-1 min-h-0">
        <Nav state={state} />
        <Scroll class="flex-1 min-w-0">
          <View class="px-6 pt-4.5 pb-6">
            <Each of={state().items} key={(i) => `${i.control ?? "heading"}:${i.key}`}>
              {(item, index) => <Row item={item} index={index} state={state} />}
            </Each>
          </View>
        </Scroll>
        <If when={width() > PREVIEW_FROM}>
          <Preview />
        </If>
      </View>
    </View>
  );
}

/** The title, a failed write of config.toml, and the close button. It is the window's title bar:
 * Settings covers the buffer line. */
function Header(props: { state: () => SettingsState }) {
  return (
    <View
      class="flex-row h-9 shrink-0 pr-2 gap-4 items-center border-b border-border"
      style={{ paddingLeft: controlsLeft() > 0 ? 0 : 24 }}
      desktop={DRAG}
    >
      <Controls side="left" />
      <Text class="text-base font-medium text-foreground" role="header">
        Settings
      </Text>
      <View class="flex-1 min-w-0">
        <If when={props.state().error}>
          <Text class="text-sm text-destructive line-clamp-1" testID="settings-error">
            Could not save config.toml.
          </Text>
        </If>
      </View>
      <Button
        accessibilityLabel="Close settings"
        testID="settings-close"
        onPress={act(() => SettingsPage.close())}
        class="size-7 items-center justify-center rounded-(--radius) hover:bg-surface-raised active:bg-muted"
      >
        <Icon name="x" size={16} class="text-muted-foreground" />
      </Button>
      <Controls side="right" />
    </View>
  );
}

/** The sections, as tabs, and the path of config.toml under them. */
function Nav(props: { state: () => SettingsState }) {
  return (
    <View class="w-57.5 shrink-0 bg-muted border-r border-border">
      <View class="flex-1 pt-1.5" role="tablist">
        <Each of={props.state().sections}>
          {(name) => {
            const on = () => props.state().section === name();
            return (
              <Button
                role="tab"
                selected={on()}
                accessibilityLabel={name()}
                testID={`section:${name().toLowerCase()}`}
                onPress={act(() => SettingsPage.showSection(name()))}
                class="flex-row h-6.5 items-center"
                // The active section keeps its look under the pointer: only the others take hover.
                classList={{ "bg-selection": on(), "bg-muted": !on(), "hover:bg-surface-raised": !on() }}
              >
                <View class="w-0.75 h-4 mr-3" classList={{ "bg-primary": on(), "bg-muted": !on() }} />
                <Text class="text-sm text-foreground">{name()}</Text>
              </Button>
            );
          }}
        </Each>
      </View>
      <View class="gap-1.5 pt-3 pb-3.5 px-4.5 border-t border-border">
        <Text class="text-xs text-muted-foreground">{props.state().configPath}</Text>
        <Button accessibilityLabel="Edit config.toml" onPress={act(() => SettingsPage.openConfig())} class="self-start hover:opacity-80">
          <Text class="text-xs text-primary">Edit config.toml</Text>
        </Button>
      </View>
    </View>
  );
}

/** One item: a heading, or an option with its control. A row keeps its key, and with it its
 * control kind, so only the values in it follow the app. */
function Row(props: { item: () => SettingsItem; index: () => number; state: () => SettingsState }) {
  const kind = props.item().control ?? "heading";
  const key = props.item().key;
  if (kind === "heading") {
    return (
      <Text class="text-xs pb-1.5 text-muted-foreground" classList={{ "pt-4.5": props.index() > 0 }}>
        {props.item().key}
      </Text>
    );
  }
  const kb = () => props.state().row === props.index();
  const label = (
    <View class="flex-1 min-w-50 gap-0.5">
      <Text class="text-sm text-foreground">{key}</Text>
      <Text class="text-xs text-muted-foreground">{props.item().desc}</Text>
    </View>
  );
  // An on / off option is one checkbox: the box, then the name and the description. The whole
  // row takes the press.
  if (kind === "check") {
    return (
      <View
        class="w-full py-1 border-b border-border"
        classList={{ "bg-surface-raised": kb(), "bg-background": !kb() }}
        testID={`row:${key}`}
      >
        <Check label={key} on={props.item().on} onPress={() => SettingsPage.toggle(key)} class="w-full min-h-10 py-1">
          {label}
        </Check>
      </View>
    );
  }
  if (kind === "themes") {
    return (
      <View
        class="w-full gap-2.5 py-2.5 border-b border-border"
        classList={{ "bg-surface-raised": kb(), "bg-background": !kb() }}
        testID={`row:${key}`}
      >
        {label}
        <Themes item={props.item} />
      </View>
    );
  }
  return (
    <View
      class="w-full py-1 border-b border-border flex-row min-h-10 gap-6 items-center flex-wrap"
      classList={{ "bg-surface-raised": kb(), "bg-background": !kb() }}
      testID={`row:${key}`}
    >
      {label}
      <Control kind={kind} item={props.item} />
    </View>
  );
}

/** The control of an option row. */
function Control(props: { kind: string; item: () => SettingsItem }) {
  const key = props.item().key;
  switch (props.kind) {
    case "choice":
      return (
        <ChoiceGroup label={key}>
          <Each of={props.item().options} key={(o) => o.value}>
            {(o, i) => (
              <Choice
                id={`${key}=${o().value}`}
                label={o().label}
                first={i() === 0}
                on={props.item().value === o().value}
                onPress={() => SettingsPage.pick(key, o().value)}
              />
            )}
          </Each>
        </ChoiceGroup>
      );
    case "font":
      return (
        <FontSelect
          name={key}
          value={props.item().value}
          family={props.item().family}
          fonts={props.item().options}
          onPick={(f) => SettingsPage.pick(key, f)}
        />
      );
    case "stepper":
      return <Stepper name={key} shown={props.item().value} onStep={(dir) => SettingsPage.step(key, dir)} />;
    case "chips":
      return <Chips colors={props.item().colors} width={11} height={22} />;
    default:
      return (
        <Text class="text-sm text-foreground" testID={`${key}=value`}>
          {props.item().value}
        </Text>
      );
  }
}

/** The themes of a row as cards: the theme's swatches and its name. */
function Themes(props: { item: () => SettingsItem }) {
  const key = props.item().key;
  return (
    <View class="flex-row flex-wrap gap-2 w-full">
      <Each of={props.item().options} key={(o) => o.value}>
        {(o) => {
          const on = () => props.item().value === o().value;
          return (
            <Button
              selected={on()}
              accessibilityLabel={o().label}
              testID={`${key}=${o().value}`}
              onPress={act(() => SettingsPage.pick(key, o().value))}
              class="flex-row w-49 h-8.5 px-2 gap-2.5 items-center rounded-(--radius)"
              classList={{ "border-2": on(), "border-primary": on(), border: !on(), "border-border": !on(), "hover:opacity-85": !on() }}
            >
              <Chips colors={o().swatches} width={7} height={18} />
              <Text class="text-sm" classList={{ "text-foreground": on(), "text-muted-foreground": !on() }}>
                {o().label}
              </Text>
            </Button>
          );
        }}
      </Each>
    </View>
  );
}
