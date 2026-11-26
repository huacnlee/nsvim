// First-launch setup: keys, mouse, look, terminal, ready. `:Setup` shows it again later, from the
// current config, with Cancel to leave it as it was. Each step is cards and checkboxes. The
// status line shows where you are and moves on. The keys work as the hint line says: Rust owns
// them and the state (`SetupFlow`).

import { signal, Text, token, View, Scroll } from "nsh";
import { Button } from "nsh/base";
import { Icon, type SetupState, SetupFlow } from "nsvim-native";
import { act } from "../controls";
import { DRAG } from "../title-bar";
import { ShellStep, ReadyStep } from "./finish";
import { KeysStep, LookStep, MouseStep } from "./steps";

/** First-launch setup, in place of the editor. */
export function Setup() {
  const state = signal<SetupState | null>(null);
  SetupFlow.onChange((s) => state.set(s));
  const vim = () => state()?.vim ?? true;
  return (
    <View class="flex-1 bg-background" testID="setup">
      {/* The window's title bar: empty, so the window controls have room and it drags. */}
      <View class="h-9 shrink-0" desktop={DRAG} testID="setup-title-bar" />
      <Scroll class="flex-1">
        <View class="w-full items-center pt-1 pb-6 px-8">
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
        <Text class="text-base font-bold text-foreground">NsVim</Text>
        <Text class="text-xs text-muted-foreground" testID="setup-step">
          {`Setup ${step()}/${props.st().steps.length} · ${props.st().steps[step() - 1] ?? ""}`}
        </Text>
      </View>
    </View>
  );
}

/** The step that shows, by its name: Windows has no terminal step. Each step has the test id
 * `setup-step-<n>`, with n as on macOS and Linux. */
function Body(props: { st: () => SetupState }) {
  const name = () => props.st().steps[props.st().step - 1];
  return (
    <>
      <If when={name() === "Keys"}>
        <KeysStep st={props.st} />
      </If>
      <If when={name() === "Mouse"}>
        <MouseStep st={props.st} />
      </If>
      <If when={name() === "Look"}>
        <LookStep st={props.st} />
      </If>
      <If when={name() === "Terminal"}>
        <ShellStep st={props.st} />
      </If>
      <If when={name() === "Ready"}>
        <ReadyStep st={props.st} />
      </If>
    </>
  );
}

/** The app's icon (assets/editor.svg), on a 128 unit grid. */
export function AppIcon(props: { size: number }) {
  const k = props.size / 128;
  // An N of round strokes: the left stem in primary, the right stem in the insert colour, and a
  // light diagonal over both. A ring in the tile colour cuts the diagonal free of the stems.
  const capsule = (cx: number, cy: number, w: number, h: number, fill: Record<string, boolean>, rotate?: string) => (
    <View
      class="absolute"
      classList={fill}
      style={{ left: (cx - w / 2) * k, top: (cy - h / 2) * k, width: w * k, height: h * k, borderRadius: (Math.min(w, h) / 2) * k, rotate }}
    />
  );
  // The diagonal runs from (39, 34) to (89, 94).
  const length = Math.hypot(50, 60);
  const angle = `${(Math.atan2(60, 50) * 180) / Math.PI}deg`;
  // The diagonal stands out from every tile: the border colour on a light tile, as on the website,
  // and the text colour on a dark one.
  const dark = () => luminance(token("muted")) < 0.5;
  return (
    <View
      class="bg-muted border border-border"
      style={{ width: props.size, height: props.size, borderRadius: 28 * k }}
      accessible
      role="image"
      accessibilityLabel="NsVim"
    >
      {capsule(39, 64, 16, 76, { "bg-primary": true })}
      {capsule(89, 64, 16, 76, { "bg-mode-insert": true })}
      {capsule(64, 64, length + 28, 28, { "bg-muted": true }, angle)}
      {capsule(64, 64, length + 16, 16, { "bg-border": !dark(), "bg-foreground": dark() }, angle)}
    </View>
  );
}

/** The relative luminance of a `#rrggbb` or `#rrggbbaa` colour, from 0 to 1. */
function luminance(hex: string): number {
  const c = (i: number) => {
    const v = parseInt(hex.slice(i, i + 2), 16) / 255;
    return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * c(1) + 0.7152 * c(3) + 0.0722 * c(5);
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
      <If when={props.st().again}>
        <StepButton label="Cancel" onPress={() => SetupFlow.cancel()} />
      </If>
      <If when={step() < last()}>
        <StepButton label="Skip" onPress={() => SetupFlow.skip()} />
      </If>
      <If when={step() > 1}>
        <StepButton label="Back" onPress={() => SetupFlow.go(-1)} />
      </If>
      <If when={step() < last()} else={<StepButton label={props.st().again ? "Save" : "Open NsVim"} go onPress={() => SetupFlow.finish()} />}>
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
