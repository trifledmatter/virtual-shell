/* tslint:disable */
/* eslint-disable */
export function set_async_result_callback(callback: Function): void;
export function get_assembly_template(template_type: string): string;
export class Terminal {
  free(): void;
  constructor();
  /**
   * initialize terminal with storage - call this immediately after creating terminal
   */
  init_with_storage(): Promise<any>;
  execute_command(input: string): any;
  get_current_directory(): string;
  list_files(path?: string | null): any;
  read_file(path: string): any;
  write_file(path: string, content: string): Promise<any>;
  get_command_list(): any;
  get_environment_variables(): any;
  set_environment_variable(key: string, value: string): void;
  is_nano_mode(): boolean;
  get_nano_filename(): string | undefined;
  process_nano_input(input: string): any;
  get_nano_editor_state(): any;
  /**
   * get storage statistics and usage info
   */
  get_storage_info(): Promise<any>;
  /**
   * manually save current vfs state (usually automatic)
   */
  manual_save(): Promise<any>;
  /**
   * manually reload vfs from storage (destructive!)
   */
  manual_reload(): Promise<any>;
  /**
   * clear all persistent storage (reset filesystem)
   */
  clear_storage(): Promise<any>;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly set_async_result_callback: (a: any) => void;
  readonly __wbg_terminal_free: (a: number, b: number) => void;
  readonly terminal_new: () => number;
  readonly terminal_init_with_storage: (a: number) => any;
  readonly terminal_execute_command: (a: number, b: number, c: number) => any;
  readonly terminal_get_current_directory: (a: number) => [number, number];
  readonly terminal_list_files: (a: number, b: number, c: number) => any;
  readonly terminal_read_file: (a: number, b: number, c: number) => any;
  readonly terminal_write_file: (a: number, b: number, c: number, d: number, e: number) => any;
  readonly terminal_get_command_list: (a: number) => any;
  readonly terminal_get_environment_variables: (a: number) => any;
  readonly terminal_set_environment_variable: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly terminal_is_nano_mode: (a: number) => number;
  readonly terminal_get_nano_filename: (a: number) => [number, number];
  readonly terminal_process_nano_input: (a: number, b: number, c: number) => any;
  readonly terminal_get_nano_editor_state: (a: number) => any;
  readonly terminal_get_storage_info: (a: number) => any;
  readonly terminal_manual_save: (a: number) => any;
  readonly terminal_manual_reload: (a: number) => any;
  readonly terminal_clear_storage: (a: number) => any;
  readonly get_assembly_template: (a: number, b: number) => [number, number];
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_4: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_export_6: WebAssembly.Table;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hb7ade31a031a667e: (a: number, b: number) => void;
  readonly closure204_externref_shim_multivalue_shim: (a: number, b: number, c: any) => [number, number];
  readonly __externref_table_dealloc: (a: number) => void;
  readonly closure256_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure919_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
