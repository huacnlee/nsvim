// The finder's rows. Rust ranks the candidates and sends only the rows on screen, from the
// list's scroll position. The list tells Rust how many rows fit, and sends it the wheel.

import { signal, Text, View, type WheelEvent } from "nsh";
import { Icon, Overlays, Shell, type PickList as Rows, type PickRow } from "nsvim-native";

import { SIZE, TERM } from "./Titled";

/** One row's height, in points: one cell in a terminal. */
const ROW_H = TERM ? 16 : 22;
/** The list's padding above and below its rows (`pt-[10px] pb-[6px]`). A terminal has none:
 * the border's cell is the gap. */
const PAD_Y = TERM ? 0 : 16;
/** The list's padding left and right of its rows, so a picked row's fill does not touch the
 * border: one cell in a terminal. */
const PAD_X = TERM ? "px-[8px]" : "px-[6px]";
/** Rows one notch of a mouse wheel scrolls. */
const WHEEL_LINES = 3;

/** A row's texts: the folder dim, the name with its matched chars bold in the accent colour,
 * then the extra text dim. */
function Line(props: { row: PickRow }) {
  return (
    <View class="flex-row flex-1 min-w-0 items-center">
      <Text class="line-clamp-1 text-muted-foreground" style={{ fontSize: SIZE }}>
        {props.row.dir}
      </Text>
      <Text class="line-clamp-1" style={{ fontSize: SIZE }}>
        <Each of={props.row.name} key="index">
          {(run) => <Text class={run().hit ? "font-bold text-primary" : "text-foreground"}>{run().text}</Text>}
        </Each>
      </Text>
      <If when={props.row.extra}>
        <Text class="line-clamp-1 pl-[12px] flex-1 min-w-0 text-muted-foreground" style={{ fontSize: SIZE }}>
          {props.row.extra}
        </Text>
      </If>
    </View>
  );
}

function Row(props: { row: PickRow; picked: boolean }) {
  return (
    <View
      class={`flex-row ${TERM ? "h-[16px]" : "h-[22px] rounded-(--radius)"} px-[8px] items-center cursor-pointer`}
      classList={{ "bg-surface-raised": props.picked, "hover:bg-surface-raised": !props.picked }}
      role="option"
      accessibilityLabel={props.row.label}
      accessibilityState={{ selected: props.picked }}
      onPress={() => {
        Shell.focusKeys();
        Overlays.pickPress(props.row.index);
      }}
    >
      <View class="w-[16.5px] shrink-0">
        <Icon name="chevronRight" size={SIZE} class="text-primary" style={{ opacity: props.picked ? 1 : 0 }} />
      </View>
      <Line row={props.row} />
      <If when={props.row.keys}>
        <Text class="shrink-0 pl-[12px] text-muted-foreground" style={{ fontSize: SIZE }}>
          {props.row.keys}
        </Text>
      </If>
    </View>
  );
}

/** How many whole rows fit in a list `height` points high, after its padding. */
function fit(height: number) {
  return Math.max(1, Math.floor((height - PAD_Y) / ROW_H));
}

/** The rows, or what the list shows when it has none (`empty`). */
export function PickList(props: { empty: string }) {
  const list = signal<Rows>({ total: 0, rows: [] });
  const sel = signal(0);
  Overlays.onPickRows((l) => list.set(l));
  Overlays.onPickSel((i) => sel.set(i));
  const wheel = (e: WheelEvent) => {
    const dy = e.deltaMode === "lines" ? e.delta.y * WHEEL_LINES * ROW_H : e.delta.y;
    if (dy !== 0) Overlays.pickWheel(dy);
  };
  return (
    <View
      class={`flex-1 min-h-0 overflow-hidden ${PAD_X} ${TERM ? "" : "pt-[10px] pb-[6px]"}`}
      onLayout={({ bounds }) => Overlays.setPickFit(fit(bounds.size.height))}
      onWheel={wheel}
    >
      <View class="flex-1" testID="picker-list">
        <If
          when={list().total > 0}
          else={
            <Text class="px-[24px] text-muted-foreground" style={{ fontSize: SIZE }}>
              {props.empty}
            </Text>
          }
        >
          <Each of={list().rows} key={(r) => r.index}>
            {(row) => <Row row={row()} picked={sel() === row().index} />}
          </Each>
        </If>
      </View>
    </View>
  );
}
