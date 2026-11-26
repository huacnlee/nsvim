// The buffer line: the project folder over the tree, then the tabs that Rust lists. It is the
// window's title bar (`./title-bar.tsx`): a press on it outside a tab drags the window. A click on
// a tab shows it. A click on its close button or a middle click closes it. A tab with unsaved
// changes shows a small square in place of the close mark. A tab dragged along the line moves to
// where it drops, as a tab of NSH's `DockArea` does. Rust owns the state (`Tabs` in
// `ext/nsvim-native/src/exports/buffer_line.rs`).

import { effect, platform, Scroll, signal, Text, View, type MouseEvent, type ViewRef } from "nsh";
import { Icon, Shell, Tabs, type BufferLineState, type TabState } from "nsvim-native";
import { Controls, controlsLeft, DRAG } from "./title-bar";

/** The font of the interface: the chrome has fixed heights, so its text follows config.toml's
 * size and not the system text size. */
type Font = { family: string; size: number };

/** Runs `f`, then gives the keys back to the editor. */
function then(f: () => void) {
  f();
  Shell.focusKeys();
}

/** The frame of a tab. The tab that shows has a raised background. The other tabs have the
 * background of the line, and a softer step of the raised background under the pointer. */
function frame(on: boolean) {
  return `flex-row shrink-0 self-stretch items-center cursor-pointer pl-3 pr-1.5 ${on ? "bg-surface-raised" : "hover:bg-surface-raised/50"}`;
}

/** One line of interface text. With `cutStart`, text that does not fit loses its start, not
 * its end. */
function Label(props: { font: () => Font; on?: boolean; text: string; class?: string; cutStart?: boolean }) {
  return (
    <Text
      allowFontScaling={false}
      class={`${props.on ? "text-foreground" : "text-muted-foreground"} ${props.class ?? ""}`}
      style={{
        fontFamily: props.font().family,
        fontSize: Math.max(9, props.font().size),
        numberOfLines: 1,
        textOverflow: props.cutStart ? "ellipsis-start" : "ellipsis",
      }}
    >
      {props.text}
    </Text>
  );
}

/** The mark of a tab: a small solid square while it has unsaved changes, else an X. The square sits in a
 * box as large as the X, so the tab keeps its width. */
function Mark(props: { font: () => Font; index: number; dirty: boolean }) {
  // A terminal has no box smaller than a cell: the square is a char there, or the button
  // would show its label ("Close") as its text.
  if (props.dirty && platform.idiom === "terminal")
    return (
      <Text class="text-warning" accessibilityLabel="Unsaved changes" testID={`tab-mark-${props.index}`}>
        ●
      </Text>
    );
  return (
    <If
      when={props.dirty}
      else={
        <Icon name="x" size={props.font().size + 1} label="Close" class="text-muted-foreground" testID={`tab-mark-${props.index}`} />
      }
    >
      <View class="items-center justify-center" style={{ width: props.font().size + 1, height: props.font().size + 1 }}>
        <View
          class="size-2 rounded-[1px] bg-warning"
          accessible
          role="image"
          accessibilityLabel="Unsaved changes"
          testID={`tab-mark-${props.index}`}
        />
      </View>
    </If>
  );
}

/** How far the pointer moves on a tab before it drags the tab: `DRAG_SLOP` of NSH's `DockArea`. */
const DRAG_SLOP = 4;

/** Where each tab is along the line, in the tabs' scroll content: from `onLayout`. */
type Span = { x: number; width: number };

/** A tab that the pointer drags: tab `from`, the pointer at `x` in the tabs' content, `dx` past
 * where it took the tab. */
type Drag = { from: number; x: number; dx: number };

/** The place among the other tabs where a tab dragged with the pointer at `x` lands: the count of
 * the other tabs whose middle is before the pointer. As `DockArea`, it counts the places without
 * the dragged tab, which is how `Tabs.moveTab` reads `to`. */
function slotOf(spans: Span[], from: number, x: number): number {
  let slot = 0;
  spans.forEach((s, i) => {
    if (i !== from && s && s.x + s.width / 2 < x) slot += 1;
  });
  return slot;
}

/** The x of the drop indicator for `slot`: the start of the other tab at that place, else the end of
 * the last other tab. */
function indicatorX(spans: Span[], from: number, slot: number): number {
  const others = spans.filter((s, i) => i !== from && s);
  if (others.length === 0) return 0;
  return slot < others.length ? others[slot].x : others[others.length - 1].x + others[others.length - 1].width;
}

/** Tab `index`. */
function Tab(props: {
  tab: () => TabState;
  index: number;
  font: () => Font;
  drag: () => Drag | null;
  onLayout: (span: Span) => void;
  onDrag: (drag: Drag | null, drop: boolean) => void;
}) {
  const i = props.index;
  // The pointer's page x and its x in the tab when the drag took the tab.
  let start = { page: 0, x: 0 };
  const dragged = () => props.drag()?.from === i;
  let node: ViewRef | undefined;
  // The tab that shows always stays in sight: when it becomes the one that shows, the line
  // scrolls just enough to bring it in.
  effect(() => {
    if (props.tab().on) node?.reveal({ horizontal: "nearest", animated: false });
  });
  return (
    <View
      ref={(n) => {
        node = n;
        if (props.tab().on) n?.reveal({ horizontal: "nearest", animated: false });
      }}
      class={`${frame(props.tab().on)} ${dragged() ? "opacity-70 z-10" : ""}`}
      style={{ transform: [{ translateX: dragged() ? (props.drag()?.dx ?? 0) : 0 }] }}
      role="tab"
      accessibilityLabel={props.tab().name}
      accessibilityState={{ selected: props.tab().on }}
      testID={`tab-${i}`}
      onPress={() => then(() => Tabs.show(i))}
      onMouseUp={(e: MouseEvent) => {
        if (e.button === "middle") then(() => Tabs.close(i));
      }}
      onLayout={({ bounds }) => props.onLayout({ x: bounds.origin.x, width: bounds.size.width })}
      // A move past a few points takes the tab, and the pointer carries it. The responder is
      // claimed only then, so a click keeps its press. The drop moves the tab.
      onMoveShouldSetResponder={(_: unknown, g: { dx: number; dy: number }) => Math.abs(g.dx) > DRAG_SLOP}
      onResponderTerminationRequest={() => false}
      onResponderGrant={(e: { pageX: number; locationX: number }) => {
        start = { page: e.pageX, x: e.locationX };
        props.onDrag({ from: i, x: 0, dx: 0 }, false);
        return true;
      }}
      onResponderMove={(e: { pageX: number }) => props.onDrag({ from: i, x: start.x, dx: e.pageX - start.page }, false)}
      onResponderRelease={(e: { pageX: number }) => props.onDrag({ from: i, x: start.x, dx: e.pageX - start.page }, true)}
      onResponderTerminate={() => props.onDrag(null, false)}
    >
      <Label font={props.font} on={props.tab().on} text={props.tab().name} />
      {/* Only the mark's square takes the click, not the tab's full height. */}
      <View
        class="ml-1.5 items-center justify-center rounded cursor-pointer hover:bg-border"
        style={{ width: props.font().size + 7, height: props.font().size + 7 }}
        role="button"
        accessibilityLabel="Close"
        testID={`tab-close-${i}`}
        onPress={() => then(() => Tabs.close(i))}
      >
        {/* Each mark is its own icon, made again only when the tab becomes dirty or clean. */}
        <If when={props.tab().dirty} else={<Mark font={props.font} index={i} dirty={false} />}>
          <Mark font={props.font} index={i} dirty />
        </If>
      </View>
    </View>
  );
}

/** The line of open files above the text, and the window's title bar. */
export function BufferLine() {
  const project = signal<BufferLineState["project"]>(undefined);
  const tabs = signal<TabState[]>([]);
  const font = signal<Font>({ family: "system-ui", size: 13 });
  // The tabs' places, for a drag, in order.
  const spans: Span[] = [];
  // Rust posts the state only when it changed. The project and the font are set only when
  // their own part changed, so a change to a tab redraws only the tabs.
  Tabs.onChange((s) => {
    const p = project();
    if (!s.project || !p || p.path !== s.project.path || p.width !== s.project.width) project.set(s.project);
    spans.length = s.tabs.length;
    tabs.set(s.tabs);
    if (font().family !== s.fontFamily || font().size !== s.fontSize) font.set({ family: s.fontFamily, size: s.fontSize });
  });
  // The drag that runs.
  const drag = signal<Drag | null>(null);
  /** The pointer's x in the tabs' content during `d`. */
  const pointer = (d: Drag) => (spans[d.from]?.x ?? 0) + d.x + d.dx;
  const slot = () => {
    const d = drag();
    return d ? slotOf(spans, d.from, pointer(d)) : null;
  };
  const onDrag = (d: Drag | null, drop: boolean) => {
    if (drop && d) {
      const to = slotOf(spans, d.from, pointer(d));
      drag.set(null);
      then(() => {
        if (to !== d.from) Tabs.moveTab(d.from, to);
      });
      return;
    }
    drag.set(d);
  };
  return (
    <View class="flex-row items-center h-9 shrink-0 overflow-hidden bg-muted" role="tablist" testID="buffer-line" desktop={DRAG}>
      <If when={project()} else={<Controls side="left" />}>
        {(p) => (
          // As wide as the tree, with the window controls at its start. The path keeps its end,
          // which tells folders apart.
          <View
            class="flex-row shrink-0 self-stretch items-center pr-3 overflow-hidden border-r border-border"
            style={{ width: p().width, paddingLeft: controlsLeft() > 0 ? controlsLeft() + 4 : 12 }}
            accessibilityLabel={p().path}
            testID="project"
          >
            <Label font={font} text={p().path} class="flex-1 min-w-0" cutStart />
          </View>
        )}
      </If>
      {/* The tabs scroll sideways when they do not fit, and stop before the window controls.
          The space after the last tab still drags the window. */}
      <Scroll
        horizontal
        scrollbar={false}
        class="flex-1 min-w-0 self-stretch"
        contentClass="flex-row grow items-stretch"
        testID="tabs"
        desktop={DRAG}
      >
        <Each of={tabs()} key="index">
          {(tab, index) => (
            <Tab
              tab={tab}
              index={index()}
              font={font}
              drag={drag}
              onLayout={(span) => (spans[index()] = span)}
              onDrag={onDrag}
            />
          )}
        </Each>
        {/* Where the dragged tab lands, while it moves to another place. */}
        <If when={slot() !== null && slot() !== drag()?.from}>
          <View
            class="absolute top-1 bottom-1 w-0.5 bg-primary"
            style={{ left: indicatorX(spans, drag()?.from ?? 0, slot() ?? 0) - 1 }}
            testID="tab-drop"
          />
        </If>
        <View class="flex-1 self-stretch" desktop={DRAG} />
      </Scroll>
      <Controls side="right" />
    </View>
  );
}
