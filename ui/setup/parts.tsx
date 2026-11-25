// The parts that the steps of setup share: the heading, the radio cards, the checkbox rows, the
// key rows and the titled boxes.

import { type JSX, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Icon, type SetupState, SetupFlow } from "nsvim/native";
import { act, Check, Titled } from "../controls";

/** The title of a step and a line under it. */
export function Heading(props: { title: string; sub: string }) {
  return (
    <View class="gap-1.5">
      <Text class="text-2xl font-bold text-foreground">{props.title}</Text>
      <Text class="text-sm text-muted-foreground">{props.sub}</Text>
    </View>
  );
}

/** A card that picks one option: a tag in its top edge, a title, a radio mark, a description
 * and a body. `choice` is the id that `SetupFlow.choose` takes. */
export function Card(props: {
  st: () => SetupState;
  choice: string;
  tag: string;
  title: string;
  desc: string;
  on: boolean;
  class?: string;
  children?: JSX.Element;
}) {
  const focus = () => props.st().focus === props.choice;
  return (
    <Button
      role="radio"
      selected={props.on}
      accessibilityLabel={props.title}
      onPress={act(() => SetupFlow.choose(props.choice))}
      class={`flex-1 min-w-65 gap-3 pt-4.5 pb-3.5 px-4.5 rounded-(--radius) bg-background hover:bg-surface-raised ${props.class ?? ""}`}
    >
      {/* The border is a child under the tag, so the tag hides the line under it. */}
      <View
        class="absolute inset-0 rounded-(--radius)"
        classList={{
          "border-2": props.on,
          "border-primary": props.on,
          border: !props.on,
          "border-text-faint": !props.on && focus(),
          "border-border": !props.on && !focus(),
        }}
      />
      <View class="absolute left-3 -top-2.25 z-10 px-1.25 bg-background">
        <Text class="text-xs" classList={{ "text-primary": props.on, "text-muted-foreground": !props.on }}>
          {props.tag}
        </Text>
      </View>
      <View class="flex-row items-center">
        <Text class="flex-1 text-base font-bold text-foreground">{props.title}</Text>
        <Icon name={props.on ? "circleDot" : "circle"} classList={{ "text-primary": props.on, "text-text-faint": !props.on }} />
      </View>
      {/* The description takes the free height, so the bodies of the cards in a row line up. */}
      <Text class="flex-1 text-xs text-muted-foreground">{props.desc}</Text>
      {props.children}
    </Button>
  );
}

/** A key and what it does, in a card. */
export function KeyRow(props: { name: string; keys: string }) {
  return (
    <View class="flex-row h-6 w-full items-center border-t border-border">
      <Text class="flex-1 text-xs text-muted-foreground">{props.name}</Text>
      <Text class="text-xs font-bold text-foreground">{props.keys}</Text>
    </View>
  );
}

/** A checkbox with a label and a line under it. The row is raised while the keyboard is on it. */
export function CheckRow(props: { st: () => SetupState; choice: string; label: string; sub?: string; on?: boolean }) {
  const name = props.choice.replace("toggle:", "");
  const on = () => props.on ?? props.st().on.includes(name);
  const focus = () => props.st().focus === props.choice;
  return (
    <View class="w-full rounded-(--radius)" classList={{ "bg-surface-raised": focus(), "bg-background": !focus() }}>
      <Check label={props.label} on={on()} onPress={() => SetupFlow.choose(props.choice)} class="w-full p-2">
        <View class="flex-1 gap-0.5">
          <Text class="text-sm text-foreground">{props.label}</Text>
          <If when={props.sub}>
            <Text class="text-xs text-muted-foreground">{props.sub}</Text>
          </If>
        </View>
      </Check>
    </View>
  );
}

/** A titled box of checkbox rows. */
export function OptionBox(props: { title: string; class?: string; children?: JSX.Element }) {
  return (
    <Titled title={props.title} class={props.class ?? "w-full"} bodyClass="pt-3.5 pb-2 px-2">
      {props.children}
    </Titled>
  );
}
