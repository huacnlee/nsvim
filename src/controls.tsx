// The styled controls of Settings and setup. They are built on `nsh/base`, which has behaviour
// and no look, so each part gets its look here, from theme tokens only.
//
// The Rust `Workspace` view takes every key. A press on a control asks Rust for a change and
// then gives the focus back to that view, so the next key goes to the editor.

import { type JSX, Text, View } from "nsh";
import { Button, Checkbox, RadioGroup, Select } from "nsh/base";
import { Icon, Shell } from "nsvim-native";

/** A press handler that runs `f`, then gives the focus back to the view that takes the keys. */
export function act(f: () => void): () => void {
  return () => {
    f();
    Shell.focusKeys();
  };
}

/** A segmented group of options, one of them picked. It has one outer border with the theme's
 * small radius, and thin lines between the segments. The group never wraps, and its labels never shrink. */
export function ChoiceGroup(props: { label: string; children?: JSX.Element }) {
  return (
    <RadioGroup
      accessibilityLabel={props.label}
      class="flex-row self-start shrink-0 h-5 rounded-(--radius) border border-border overflow-hidden"
    >
      {props.children}
    </RadioGroup>
  );
}

/** One segment of a `ChoiceGroup`. The picked one fills with the accent of a checked box. `first`
 * drops the line on the left of the first segment. `focus` marks the option the keyboard is
 * on. */
export function Choice(props: { id: string; label: string; on: boolean; first: boolean; focus?: boolean; onPress: () => void }) {
  return (
    <Button
      role="radio"
      selected={props.on}
      accessibilityLabel={props.label}
      testID={props.id}
      onPress={act(() => props.onPress())}
      class="flex-row shrink-0 self-stretch px-2.5 items-center justify-center border-border"
      classList={{
        "border-l": !props.first,
        "bg-mode-insert": props.on,
        "bg-surface-raised": !props.on && !!props.focus,
        "hover:bg-surface-raised": !props.on,
        "active:bg-muted": !props.on,
      }}
    >
      <Text
        class="shrink-0 text-xs leading-4"
        classList={{
          "text-primary-foreground": props.on,
          "text-foreground": !props.on && !!props.focus,
          "text-muted-foreground": !props.on && !props.focus,
        }}
      >
        {props.label}
      </Text>
    </Button>
  );
}

/** An on / off option: a square box with a check mark while it is on, then its content (the
 * label) to the right. The whole row takes the press. `label` is what screen readers say and
 * what tests select it by. */
export function Check(props: { label: string; on: boolean; onPress: () => void; class?: string; children?: JSX.Element }) {
  return (
    <Checkbox
      checked={props.on}
      onChange={act(() => props.onPress())}
      accessibilityLabel={props.label}
      testID={props.label}
      class={`flex-row items-center gap-3 rounded-(--radius) hover:bg-surface-raised focus:bg-surface-raised disabled:opacity-50 ${props.class ?? ""}`}
      indicatorClass="size-3.5 shrink-0 items-center justify-center rounded-(--radius) border border-border bg-background checked:border-mode-insert checked:bg-mode-insert"
      indicator={
        <If when={props.on}>
          <Icon name="check" size={12} strokeWidth={3} class="text-primary-foreground" />
        </If>
      }
    >
      {props.children}
    </Checkbox>
  );
}

/** A number with a step down and a step up button around it. */
export function Stepper(props: { name: string; shown: string; onStep: (dir: number) => void }) {
  const half = (sign: string, verb: string, dir: number) => (
    <Button
      accessibilityLabel={`${verb} ${props.name}`}
      testID={`${props.name}${sign}`}
      onPress={act(() => props.onStep(dir))}
      class="w-7 h-6 items-center justify-center hover:bg-surface-raised active:opacity-70"
    >
      <Icon name={dir < 0 ? "minus" : "plus"} size={14} class="text-muted-foreground" />
    </Button>
  );
  return (
    <View class="flex-row h-6.5 items-center rounded-(--radius) border border-border">
      {half("-", "Decrease", -1)}
      <View class="min-w-16 items-center">
        <Text class="text-sm text-foreground" testID={`${props.name}=value`}>
          {props.shown}
        </Text>
      </View>
      {half("+", "Increase", 1)}
    </View>
  );
}

/** A font in a list. `family` is the font family list that shows it in its own font. */
export type FontOption = { value: string; label: string; family: string };

/** A pick of one font from a list. The trigger and each item show the font in itself. */
export function FontSelect(props: { name: string; value: string; family: string; fonts: FontOption[]; onPick: (font: string) => void }) {
  return (
    <Select
      items={props.fonts.map((f) => ({ value: f.value, title: f.label }))}
      value={props.value}
      onChange={({ value }) => {
        if (value) act(() => props.onPick(value))();
      }}
      onClose={() => Shell.focusKeys()}
      accessibilityLabel={props.name}
      testID={props.name}
      offset={4}
      popupClass="p-1 rounded-(--radius) border border-border bg-popover shadow-md"
      itemClass="rounded-(--radius) focus:bg-accent selected:font-medium"
      renderItem={(item) => (
        <View class="px-2 py-1.5">
          <Text class="text-sm text-popover-foreground" style={{ fontFamily: props.fonts[item.index]?.family }}>
            {item.title}
          </Text>
        </View>
      )}
      trigger={
        <View class="flex-row h-7.5 min-w-50 px-3 gap-2 items-center rounded-(--radius) border border-border bg-background">
          <Text class="flex-1 text-sm text-foreground" style={{ fontFamily: props.family }}>
            {props.value}
          </Text>
          <Icon name="chevronDown" size={14} class="text-muted-foreground" />
        </View>
      }
    />
  );
}

/** A row of colour chips. The colours are data (a theme's swatches), not roles: the first chip
 * is twice as wide as the others. */
export function Chips(props: { colors: string[]; width: number; height: number }) {
  return (
    <View class="flex-row rounded-(--radius) overflow-hidden border border-border">
      <Each of={props.colors} key="index">
        {(c, i) => <View style={{ width: i() === 0 ? props.width * 2 : props.width, height: props.height, backgroundColor: c() }} />}
      </Each>
    </View>
  );
}

/** A box with a title in its top edge. `class` sizes and places the box, `bodyClass` lays out
 * its content. The content is clipped to the box. The frame paints over the content, and the
 * title over the frame, so neither the content nor a view's own border (which paints over its
 * children on GTK) can break the line or cross the title. */
export function Titled(props: { title: string; class?: string; bodyClass?: string; testID?: string; children?: JSX.Element }) {
  return (
    <View class={props.class} testID={props.testID}>
      <View class={`flex-1 rounded-(--radius) overflow-hidden bg-background ${props.bodyClass ?? ""}`}>{props.children}</View>
      <View class="absolute inset-0 z-10 rounded-(--radius) border border-border pointer-events-none" />
      <View class="absolute left-3 -top-2.25 z-20 px-1.25 bg-background pointer-events-none">
        <Text class="text-xs text-muted-foreground">{props.title}</Text>
      </View>
    </View>
  );
}

/** A key cap: `SPC`, `Ctrl`, `gd`. */
export function Kbd(props: { children: string }) {
  return (
    <View class="h-5 min-w-5 px-1.25 items-center justify-center rounded-(--radius) bg-surface-raised border border-border">
      <Text class="text-xs text-foreground">{props.children}</Text>
    </View>
  );
}

/** A small filled label: a mode name or a modifier key. `fill` and `ink` are a `bg-*` and a
 * `text-*` token class. */
export function Pill(props: { children: string; fill: string; ink: string }) {
  return (
    <View class={`px-1.75 py-0.5 rounded-(--radius) ${props.fill}`}>
      <Text class={`text-xs font-bold ${props.ink}`}>{props.children}</Text>
    </View>
  );
}
