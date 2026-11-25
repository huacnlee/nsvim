// The buffer line: the project folder over the tree, a tab for each open file, and the
// Settings tab while it is open. A click on a tab shows its file. A click on its close button
// or a middle click closes it. A file with unsaved changes shows a dot in place of the close
// mark. Rust owns the state (`Tabs` in `nsh/src/exports/buffer_line.rs`).

import { signal, Text, View, type MouseEvent } from "nsh";
import { Icon, Shell, Tabs, type BufferLineState, type TabState } from "nsvim/native";

/** The line width that fills the `dot` icon to a solid dot, of its 24 units. */
const DOT_STROKE = 9;

/** The font of the interface: the chrome has fixed heights, so its text follows config.toml's
 * size and not the system text size. */
type Font = { family: string; size: number };

/** Runs `f`, then gives the keys back to the editor. */
function then(f: () => void) {
  f();
  Shell.focusKeys();
}

/** The frame of a tab: a line in the accent colour along the bottom of the tab that shows. A
 * clear line of the same width at the top keeps the name in the middle. */
function frame(on: boolean) {
  return `flex-row shrink-0 self-stretch items-center cursor-pointer pl-3 border-t-2 border-b-2 border-t-transparent ${
    on ? "border-b-primary bg-background" : "border-b-transparent"
  }`;
}

/** One line of interface text. */
function Label(props: { font: () => Font; on?: boolean; text: string; class?: string }) {
  return (
    <Text
      allowFontScaling={false}
      class={`${props.on ? "text-foreground" : "text-muted-foreground"} ${props.class ?? ""}`}
      style={{ fontFamily: props.font().family, fontSize: Math.max(9, props.font().size), numberOfLines: 1 }}
    >
      {props.text}
    </Text>
  );
}

/** The mark on the close button: a dot while the file has unsaved changes, else an X. */
function Mark(props: { font: () => Font; index: number; dirty: boolean }) {
  return (
    <Icon
      name={props.dirty ? "dot" : "x"}
      strokeWidth={props.dirty ? DOT_STROKE : 2}
      size={props.font().size + 1}
      label={props.dirty ? "Unsaved changes" : "Close"}
      class={props.dirty ? "text-mode-command" : "text-muted-foreground"}
      testID={`tab-mark-${props.index}`}
    />
  );
}

/** The tab of file `index`. */
function Tab(props: { tab: () => TabState; index: number; font: () => Font }) {
  const i = props.index;
  return (
    <View
      class={frame(props.tab().on)}
      role="tab"
      accessibilityLabel={props.tab().name}
      accessibilityState={{ selected: props.tab().on }}
      testID={`tab-${i}`}
      onPress={() => then(() => Tabs.show(i))}
      onMouseUp={(e: MouseEvent) => {
        if (e.button === "middle") then(() => Tabs.close(i));
      }}
    >
      <Label font={props.font} on={props.tab().on} text={props.tab().name} />
      <View
        class="self-stretch justify-center px-2.5 cursor-pointer hover:bg-surface-raised"
        role="button"
        accessibilityLabel="Close"
        testID={`tab-close-${i}`}
        onPress={() => then(() => Tabs.close(i))}
      >
        {/* Each mark is its own icon, made again only when the file becomes dirty or clean. */}
        <If when={props.tab().dirty} else={<Mark font={props.font} index={i} dirty={false} />}>
          <Mark font={props.font} index={i} dirty />
        </If>
      </View>
    </View>
  );
}

/** The line of open files above the text. */
export function BufferLine() {
  const project = signal<BufferLineState["project"]>(undefined);
  const tabs = signal<TabState[]>([]);
  const settings = signal<BufferLineState["settings"]>(undefined);
  const font = signal<Font>({ family: "system-ui", size: 13 });
  // Rust posts the state only when it changed. The project, the Settings tab and the font
  // are set only when their own part changed, so a change to a tab redraws only the tabs.
  Tabs.onChange((s) => {
    const p = project();
    if (!s.project || !p || p.path !== s.project.path || p.width !== s.project.width) project.set(s.project);
    tabs.set(s.tabs);
    const t = settings();
    if (!s.settings || !t || t.on !== s.settings.on || t.dirty !== s.settings.dirty) settings.set(s.settings);
    if (font().family !== s.fontFamily || font().size !== s.fontSize) font.set({ family: s.fontFamily, size: s.fontSize });
  });
  return (
    <View class="flex-row items-center h-7 shrink-0 overflow-hidden bg-muted" role="tablist" testID="buffer-line">
      <If when={project()}>
        {(p) => (
          <View
            class="flex-row shrink-0 self-stretch items-center px-3 border-r border-border"
            style={{ width: p().width }}
            testID="project"
          >
            <Label font={font} text={p().path} class="truncate" />
          </View>
        )}
      </If>
      <Each of={tabs()} key="index">
        {(tab, index) => <Tab tab={tab} index={index()} font={font} />}
      </Each>
      <If when={settings()}>
        {(t) => (
          <View
            class={`${frame(t().on)} pr-3`}
            role="tab"
            accessibilityLabel="settings"
            accessibilityState={{ selected: t().on }}
            onPress={() => then(() => Tabs.showSettings())}
          >
            <Label font={font} on={t().on} text={t().dirty ? "settings [+]" : "settings"} />
          </View>
        )}
      </If>
    </View>
  );
}
