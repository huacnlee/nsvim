// First-launch setup: keys, mouse, look, terminal, ready. Each step is cards and switches. The
// status line shows where you are and moves on. The keys work as the hint line says: Rust owns
// them and the state (`SetupFlow`).

import { signal, Text, View, Scroll } from "nsh";
import { Button } from "nsh/base";
import { Icon, type SetupState, SetupFlow } from "nsvim/native";
import { act } from "../controls";
import { ShellStep, ReadyStep } from "./finish";
import { KeysStep, LookStep, MouseStep } from "./steps";

/** First-launch setup, in place of the editor. */
export function Setup() {
  const state = signal<SetupState | null>(null);
  SetupFlow.onChange((s) => state.set(s));
  const vim = () => state()?.vim ?? true;
  return (
    <View class="flex-1 bg-background" testID="setup">
      <Scroll class="flex-1">
        <View class="w-full items-center pt-10 pb-6 px-8">
          <View class="w-full max-w-225 gap-6.5">
            <If when={state()}>
              {(st) => (
                <>
                  <Header st={st} />
                  <Body st={st} />
                </>
              )}
            </If>
          </View>
        </View>
      </Scroll>
      <If when={state()}>{(st) => <Status st={st} />}</If>
      <View class="flex-row h-5.5 px-2 items-center">
        <Text class="text-xs text-muted-foreground">{vim() ? "j/k move   Enter picks   Tab next" : "Arrows move   Enter picks   Tab next"}</Text>
      </View>
    </View>
  );
}

function Header(props: { st: () => SetupState }) {
  const step = () => props.st().step;
  return (
    <View class="flex-row gap-3.5 items-center">
      <AppIcon size={34} />
      <View class="gap-0.5">
        <Text class="text-base font-bold text-foreground">nsvim</Text>
        <Text class="text-xs text-muted-foreground" testID="setup-step">
          {`Setup ${step()}/${props.st().steps.length} · ${props.st().steps[step() - 1] ?? ""}`}
        </Text>
      </View>
    </View>
  );
}

/** The step that shows. Each step has the test id `setup-step-<n>`. */
function Body(props: { st: () => SetupState }) {
  const step = () => props.st().step;
  return (
    <>
      <If when={step() === 1}>
        <KeysStep st={props.st} />
      </If>
      <If when={step() === 2}>
        <MouseStep st={props.st} />
      </If>
      <If when={step() === 3}>
        <LookStep st={props.st} />
      </If>
      <If when={step() === 4}>
        <ShellStep st={props.st} />
      </If>
      <If when={step() >= 5}>
        <ReadyStep st={props.st} />
      </If>
    </>
  );
}

/** The app's icon: four lines in the mode colours, on a 128 unit grid. */
export function AppIcon(props: { size: number }) {
  const k = props.size / 128;
  const bar = (x: number, y: number, w: number, fill: string) => (
    <View class={`absolute ${fill}`} style={{ left: x * k, top: y * k, width: w * k, height: 12 * k, borderRadius: 6 * k }} />
  );
  return (
    <View
      class="bg-muted border border-border"
      style={{ width: props.size, height: props.size, borderRadius: 28 * k }}
      accessible
      role="image"
      accessibilityLabel="nsvim"
    >
      {bar(28, 32, 72, "bg-primary")}
      {bar(28, 52, 50, "bg-mode-insert")}
      {bar(40, 72, 60, "bg-mode-visual")}
      {bar(28, 92, 34, "bg-mode-command")}
    </View>
  );
}

/** The status line: SETUP, the steps, and skip, back and next. */
function Status(props: { st: () => SetupState }) {
  const step = () => props.st().step;
  const last = () => props.st().steps.length;
  return (
    <View class="flex-row h-6 w-full items-center bg-muted">
      <View class="self-stretch justify-center px-3 bg-primary">
        <Text class="text-xs font-bold text-primary-foreground">SETUP</Text>
      </View>
      <Each of={props.st().steps} key="index">
        {(name, i) => {
          const n = () => i() + 1;
          // Each key of a `classList` is its own binding. A whole object built by a call is not
          // followed on a Rust component such as `Icon`.
          return (
            <View class="flex-row items-center">
              <If when={i() > 0}>
                <Icon
                  name="chevronRight"
                  size={12}
                  class="ml-1.5"
                  classList={{
                    "text-foreground": n() === step(),
                    "text-mode-insert": n() < step(),
                    "text-muted-foreground": n() > step(),
                  }}
                />
              </If>
              <Text
                class="text-xs px-1.5"
                classList={{
                  "text-foreground": n() === step(),
                  "text-mode-insert": n() < step(),
                  "text-muted-foreground": n() > step(),
                  "font-bold": n() === step(),
                }}
              >
                {name()}
              </Text>
            </View>
          );
        }}
      </Each>
      <View class="flex-1" />
      <If when={step() < last()}>
        <StepButton label="Skip" onPress={() => SetupFlow.skip()} />
      </If>
      <If when={step() > 1}>
        <StepButton label="Back" onPress={() => SetupFlow.go(-1)} />
      </If>
      <If when={step() < last()} else={<StepButton label="Open nsvim" go onPress={() => SetupFlow.finish()} />}>
        <StepButton label="Next" go onPress={() => SetupFlow.go(1)} />
      </If>
    </View>
  );
}

/** A button of the status line. A button that goes forward is filled and shows an arrow. */
function StepButton(props: { label: string; go?: boolean; onPress: () => void }) {
  const ink = props.go ? "text-primary-foreground" : "text-muted-foreground";
  return (
    <Button
      accessibilityLabel={props.label}
      onPress={act(() => props.onPress())}
      class="self-stretch flex-row px-3 gap-1.5 items-center hover:opacity-85 active:opacity-70"
      classList={{ "bg-primary": !!props.go, "bg-muted": !props.go }}
    >
      <Text class={`text-xs ${ink}`}>{props.label}</Text>
      <If when={props.go}>
        <Icon name="arrowRight" size={14} class={ink} />
      </If>
    </Button>
  );
}
