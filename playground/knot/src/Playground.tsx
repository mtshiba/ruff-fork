import {
  Suspense,
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
} from "react";
import { ErrorMessage, Header, setupMonaco, useTheme } from "shared";
import { FileHandle, Workspace } from "red_knot_wasm";
import { persist, persistLocal, restore } from "./Editor/persist";
import { loader } from "@monaco-editor/react";
import knotSchema from "../../../knot.schema.json";
import Chrome, { formatError } from "./Editor/Chrome";

export const SETTINGS_FILE_NAME = "knot.json";

export default function Playground() {
  const [theme, setTheme] = useTheme();
  const [version, setVersion] = useState<string>("0.0.0");
  const [error, setError] = useState<string | null>(null);
  const workspacePromiseRef = useRef<Promise<Workspace> | null>(null);

  let workspacePromise = workspacePromiseRef.current;
  if (workspacePromise == null) {
    workspacePromiseRef.current = workspacePromise = startPlayground().then(
      (fetched) => {
        setVersion(fetched.version);
        const workspace = new Workspace("/", {});

        let hasSettings = false;

        for (const [name, content] of Object.entries(fetched.workspace.files)) {
          let handle = null;
          if (name === SETTINGS_FILE_NAME) {
            updateOptions(workspace, content, setError);
            hasSettings = true;
          } else {
            handle = workspace.openFile(name, content);
          }

          dispatchFiles({ type: "add", handle, content, name });
        }

        if (!hasSettings) {
          updateOptions(workspace, null, setError);
        }

        dispatchFiles({
          type: "selectFileByName",
          name: fetched.workspace.current,
        });

        return workspace;
      },
    );
  }

  const [files, dispatchFiles] = useReducer(filesReducer, {
    index: [],
    contents: Object.create(null),
    handles: Object.create(null),
    nextId: 0,
    revision: 0,
    selected: null,
  });

  const fileName = useMemo(() => {
    return (
      files.index.find((file) => file.id === files.selected)?.name ?? "lib.py"
    );
  }, [files.index, files.selected]);

  usePersistLocally(files);

  const handleShare = useCallback(() => {
    const serialized = serializeFiles(files);

    if (serialized != null) {
      persist(serialized).catch((error) => {
        // eslint-disable-next-line no-console
        console.error("Failed to share playground", error);
      });
    }
  }, [files]);

  const handleFileAdded = (workspace: Workspace, name: string) => {
    let handle = null;

    if (name === SETTINGS_FILE_NAME) {
      updateOptions(workspace, "{}", setError);
    } else {
      handle = workspace.openFile(name, "");
    }

    dispatchFiles({ type: "add", name, handle, content: "" });
  };

  const handleFileChanged = (workspace: Workspace, content: string) => {
    if (files.selected == null) {
      return;
    }

    dispatchFiles({
      type: "change",
      id: files.selected,
      content,
    });

    const handle = files.handles[files.selected];

    if (handle != null) {
      updateFile(workspace, handle, content, setError);
    } else if (fileName === SETTINGS_FILE_NAME) {
      updateOptions(workspace, content, setError);
    }
  };

  const handleFileRenamed = (
    workspace: Workspace,
    file: FileId,
    newName: string,
  ) => {
    const handle = files.handles[file];
    let newHandle: FileHandle | null = null;
    if (handle == null) {
      updateOptions(workspace, null, setError);
    } else {
      workspace.closeFile(handle);
    }

    if (newName === SETTINGS_FILE_NAME) {
      updateOptions(workspace, files.contents[file], setError);
    } else {
      newHandle = workspace.openFile(newName, files.contents[file]);
    }

    dispatchFiles({ type: "rename", id: file, to: newName, newHandle });
  };

  const handleFileRemoved = (workspace: Workspace, file: FileId) => {
    const handle = files.handles[file];
    if (handle == null) {
      updateOptions(workspace, null, setError);
    } else {
      workspace.closeFile(handle);
    }

    dispatchFiles({ type: "remove", id: file });
  };

  const handleFileSelected = useCallback((file: FileId) => {
    dispatchFiles({ type: "selectFile", id: file });
  }, []);

  return (
    <main className="flex flex-col h-full bg-ayu-background dark:bg-ayu-background-dark">
      <Header
        edit={files.revision}
        theme={theme}
        logo="astral"
        version={version}
        onChangeTheme={setTheme}
        onShare={handleShare}
      />

      <Suspense fallback={<Loading />}>
        <Chrome
          files={files}
          workspacePromise={workspacePromise}
          theme={theme}
          selectedFileName={fileName}
          onFileAdded={handleFileAdded}
          onFileRenamed={handleFileRenamed}
          onFileRemoved={handleFileRemoved}
          onFileSelected={handleFileSelected}
          onFileChanged={handleFileChanged}
        />
      </Suspense>
      {error ? (
        <div
          style={{
            position: "fixed",
            left: "10%",
            right: "10%",
            bottom: "10%",
          }}
        >
          <ErrorMessage>{error}</ErrorMessage>
        </div>
      ) : null}
    </main>
  );
}

export const DEFAULT_SETTINGS = JSON.stringify(
  {
    environment: {
      "python-version": "3.13",
    },
    rules: {
      "division-by-zero": "error",
    },
  },
  null,
  4,
);

/**
 * Persists the files to local storage. This is done deferred to avoid too frequent writes.
 */
function usePersistLocally(files: FilesState): void {
  const deferredFiles = useDeferredValue(files);

  useEffect(() => {
    const serialized = serializeFiles(deferredFiles);
    if (serialized != null) {
      persistLocal(serialized);
    }
  }, [deferredFiles]);
}

export type FileId = number;

export type ReadonlyFiles = Readonly<FilesState>;

interface FilesState {
  /**
   * The currently selected file that is shown in the editor.
   */
  selected: FileId | null;

  /**
   * The files in display order (ordering is sensitive)
   */
  index: ReadonlyArray<{ id: FileId; name: string }>;

  /**
   * The database file handles by file id.
   *
   * Files without a file handle are well-known files that are only handled by the
   * playground (e.g. knot.json)
   */
  handles: Readonly<{ [id: FileId]: FileHandle | null }>;

  /**
   * The content per file indexed by file id.
   */
  contents: Readonly<{ [id: FileId]: string }>;

  /**
   * The revision. Gets incremented every time files changes.
   */
  revision: number;
  nextId: FileId;
}

export type FileAction =
  | {
      type: "add";
      handle: FileHandle | null;
      /// The file name
      name: string;
      content: string;
    }
  | {
      type: "change";
      id: FileId;
      content: string;
    }
  | { type: "rename"; id: FileId; to: string; newHandle: FileHandle | null }
  | {
      type: "remove";
      id: FileId;
    }
  | { type: "selectFile"; id: FileId }
  | { type: "selectFileByName"; name: string };

function filesReducer(
  state: Readonly<FilesState>,
  action: FileAction,
): FilesState {
  switch (action.type) {
    case "add": {
      const { handle, name, content } = action;
      const id = state.nextId;
      return {
        ...state,
        selected: id,
        index: [...state.index, { id, name }],
        handles: { ...state.handles, [id]: handle },
        contents: { ...state.contents, [id]: content },
        nextId: state.nextId + 1,
        revision: state.revision + 1,
      };
    }

    case "change": {
      const { id, content } = action;
      return {
        ...state,
        contents: { ...state.contents, [id]: content },
        revision: state.revision + 1,
      };
    }

    case "remove": {
      const { id } = action;

      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { [id]: _content, ...contents } = state.contents;
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { [id]: _handle, ...handles } = state.handles;

      let selected = state.selected;

      if (state.selected === id) {
        const index = state.index.findIndex((file) => file.id === id);

        selected =
          index > 0 ? state.index[index - 1].id : state.index[index + 1].id;
      }

      return {
        ...state,
        selected,
        index: state.index.filter((file) => file.id !== id),
        contents,
        handles,
        revision: state.revision + 1,
      };
    }
    case "rename": {
      const { id, to, newHandle } = action;

      const index = state.index.findIndex((file) => file.id === id);
      const newIndex = [...state.index];
      newIndex.splice(index, 1, { id, name: to });

      return {
        ...state,
        index: newIndex,
        handles: { ...state.handles, [id]: newHandle },
      };
    }

    case "selectFile": {
      const { id } = action;

      return {
        ...state,
        selected: id,
      };
    }

    case "selectFileByName": {
      const { name } = action;

      const selected =
        state.index.find((file) => file.name === name)?.id ?? null;

      return {
        ...state,
        selected,
      };
    }
  }
}

function serializeFiles(files: FilesState): {
  files: { [name: string]: string };
  current: string;
} | null {
  const serializedFiles = Object.create(null);
  let selected = null;

  for (const { id, name } of files.index) {
    serializedFiles[name] = files.contents[id];

    if (files.selected === id) {
      selected = name;
    }
  }

  if (selected == null) {
    return null;
  }

  return { files: serializedFiles, current: selected };
}

export interface InitializedPlayground {
  version: string;
  workspace: { files: { [name: string]: string }; current: string };
}

// Run once during startup. Initializes monaco, loads the wasm file, and restores the previous editor state.
async function startPlayground(): Promise<InitializedPlayground> {
  const red_knot = await import("../red_knot_wasm");
  await red_knot.default();
  const monaco = await loader.init();

  setupMonaco(monaco, {
    uri: "https://raw.githubusercontent.com/astral-sh/ruff/main/knot.schema.json",
    fileMatch: ["knot.json"],
    schema: knotSchema,
  });

  const restored = await restore();

  const workspace = restored ?? {
    files: {
      "main.py": "import os",
      "knot.json": DEFAULT_SETTINGS,
    },
    current: "main.py",
  };

  return {
    version: "0.0.0",
    workspace,
  };
}

function updateOptions(
  workspace: Workspace | null,
  content: string | null,
  setError: (error: string | null) => void,
) {
  content = content ?? DEFAULT_SETTINGS;

  try {
    const settings = JSON.parse(content);
    workspace?.updateOptions(settings);
    setError(null);
  } catch (error) {
    setError(`Failed to update 'knot.json' options: ${formatError(error)}`);
  }
}

function updateFile(
  workspace: Workspace,
  handle: FileHandle,
  content: string,
  setError: (error: string | null) => void,
) {
  try {
    workspace.updateFile(handle, content);
    setError(null);
  } catch (error) {
    setError(`Failed to update file: ${formatError(error)}`);
  }
}

function Loading() {
  return <div className="align-middle  text-center my-2">Loading...</div>;
}
