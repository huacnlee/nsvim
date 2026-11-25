// The file tree: the project's folders and files, folders first. A press opens a folder or
// closes it, or opens a file. The wheel scrolls it. A drag on its right edge changes its width.
//
// Rust owns the tree (`FileTree` in `nsh/src/exports/tree.rs`). Rust keeps the scroll position
// and gives only the rows on screen, so a tree of 200,000 files costs the same as a small one.
// The tree's keys go through `App::key`. After a press, the keys go back to the editor.

import { signal, Text, View, type Signal } from "nsh";
import { FileTree, Icon, Shell, type TreeFrame, type TreeItem } from "nsvim/native";

/** The frame before Rust gives the first one. */
const NO_FRAME: TreeFrame = { width: 260, font: "system-ui", size: 12.5, project: "" };

/** The frame of the tree, as a signal that follows Rust. */
function frameSignal(): Signal<TreeFrame> {
  const frame = signal<TreeFrame>(NO_FRAME);
  FileTree.onFrame((f) => frame.set(f));
  return frame;
}

/** The style of one line of interface text. The text follows config.toml's size, not the
 * system text size. */
function font(frame: Signal<TreeFrame>) {
  return { fontFamily: frame().font, fontSize: Math.max(frame().size, 9), numberOfLines: 1 };
}

/** The class of a row's name: the accent for folders, else the colour of its git status. */
function nameClass(r: TreeItem): string {
  if (r.dir) return "text-primary";
  switch (r.git) {
    case "U":
    case "A":
      return "text-mode-insert";
    case "M":
      return "text-mode-command";
    case "D":
      return "text-destructive";
    default:
      return "text-foreground";
  }
}

/** The class of a git mark: the letter of a file, or the dot of a folder. */
function gitClass(r: TreeItem): string {
  switch (r.git) {
    case "U":
    case "A":
      return "text-mode-insert";
    case "D":
      return "text-destructive";
    default:
      return "text-mode-command";
  }
}

/** The class of a row's fill: the keyboard row, then the open file. */
function rowClass(r: TreeItem): string {
  if (r.picked) return "bg-selection";
  if (r.active) return "bg-selection/60";
  return "";
}

/** Whether two items show the same. */
function same(a: TreeItem, b: TreeItem): boolean {
  return (
    a.path === b.path &&
    a.name === b.name &&
    a.depth === b.depth &&
    a.dir === b.dir &&
    a.open === b.open &&
    a.active === b.active &&
    a.picked === b.picked &&
    a.git === b.git &&
    a.changes === b.changes
  );
}

/** One row. Rows are kept by position: a scroll changes the item of each row, and makes no
 * new rows. */
function Row(props: { item: () => TreeItem; frame: Signal<TreeFrame> }) {
  const r = () => props.item();
  const size = () => props.frame().size;
  return (
    <View
      // Rust counts the rows that fit with the same height, 22 points.
      class={`h-[22px] flex-row items-center pl-2 pr-3 hover:bg-surface-raised ${rowClass(r())}`}
      style={{ cursor: "pointer" }}
      role="treeitem"
      accessibilityLabel={r().name}
      accessibilityLevel={r().depth + 1}
      accessibilityState={{ selected: r().picked || r().active, expanded: r().dir ? r().open : undefined }}
      onPress={() => {
        FileTree.press(r().path, r().dir);
        Shell.focusKeys();
      }}
    >
      <Text class="text-border" style={font(props.frame)} allowFontScaling={false}>
        {"│ ".repeat(r().depth)}
      </Text>
      {/* A folder shows a chevron, a file the same space empty, so the names line up. */}
      <View class="shrink-0 text-muted-foreground" style={{ width: size() + 5 }}>
        <If when={r().dir}>
          <If when={r().open} else={<Icon name="chevronRight" size={size()} testID="tree-closed" />}>
            <Icon name="chevronDown" size={size()} testID="tree-open" />
          </If>
        </If>
      </View>
      <Text class={`shrink truncate ${nameClass(r())}`} style={font(props.frame)} allowFontScaling={false}>
        {r().name}
      </Text>
      <View class="flex-1" />
      <If when={r().changes}>
        {/* A folder with changes inside shows a solid dot. */}
        <View class={`pl-2 ${gitClass(r())}`}>
          <Icon name="dot" strokeWidth={9} size={size()} testID="tree-changes" />
        </View>
      </If>
      <If when={r().git}>
        <Text class={`pl-2 ${gitClass(r())}`} style={font(props.frame)} allowFontScaling={false}>
          {r().git ?? ""}
        </Text>
      </If>
    </View>
  );
}

/** The tree's right edge: a drag changes the tree's width, and the release keeps it in the
 * config. */
function Edge(props: { frame: Signal<TreeFrame> }) {
  let start = { x: 0, width: 0 };
  const resize = (pageX: number, done: boolean) => FileTree.resize(start.width + pageX - start.x, done);
  return (
    <View
      class="absolute top-0 bottom-0 w-[6px]"
      style={{ left: props.frame().width - 3, cursor: "col-resize" }}
      testID="tree-edge"
      onStartShouldSetResponder={() => true}
      onResponderGrant={(e) => {
        start = { x: e.pageX, width: props.frame().width };
        return true;
      }}
      onResponderMove={(e) => resize(e.pageX, false)}
      onResponderRelease={(e) => resize(e.pageX, true)}
    />
  );
}

/** The project folder over the tree, as wide as the tree. It goes first in the buffer line. */
export function TreeHeader() {
  const frame = frameSignal();
  return (
    <View class="flex-row shrink-0 self-stretch items-center px-3 border-r border-border" style={{ width: frame().width }} testID="project">
      <Text class="truncate text-muted-foreground" style={font(frame)} allowFontScaling={false}>
        {frame().project}
      </Text>
    </View>
  );
}

/** The file tree beside the text. */
export function Tree() {
  const frame = frameSignal();
  const rows = signal<TreeItem[]>([]);
  // A row whose item stays equal keeps the old object, so a key that moves the keyboard row
  // updates two rows and not all of them.
  FileTree.onRows((next) => {
    const old = rows();
    rows.set(next.rows.map((r, i) => (old[i] !== undefined && same(old[i], r) ? old[i] : r)));
  });
  return (
    <View
      class="shrink-0 self-stretch min-h-0 overflow-hidden bg-muted border-r border-border"
      style={{ width: frame().width }}
      role="tree"
      accessibilityLabel="Files"
      testID="tree"
      onWheel={(e) => FileTree.wheel(e.delta.y, e.deltaMode === "lines")}
    >
      <View class="pt-1">
        <Each of={rows()} key="index">
          {(item) => <Row item={item} frame={frame} />}
        </Each>
      </View>
      <Edge frame={frame} />
    </View>
  );
}
