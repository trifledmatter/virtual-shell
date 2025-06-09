// TypeScript declarations for the WASM terminal module

export interface CommandResponse {
  success: boolean;
  output: string;
}

export interface FileInfo {
  name: string;
  type: 'file' | 'directory';
  size: number;
  permissions: string;
}

export interface ListFilesResponse {
  success: boolean;
  files?: FileInfo[];
  error?: string;
}

export interface ReadFileResponse {
  success: boolean;
  content?: string;
  error?: string;
}

export interface WriteFileResponse {
  success: boolean;
  error?: string;
}

export interface NanoResponse {
  success: boolean;
  message?: string;
  output?: string;
  error?: string;
  continue_editing?: boolean;
  exit_nano?: boolean;
  refresh_content?: boolean;
  refresh?: boolean;
  exit?: boolean;
  prompt_save?: boolean;
}

export interface SyntaxHighlight {
  start: number;
  end: number;
  type: 'instruction' | 'comment' | 'label' | 'number' | 'keyword' | 'builtin' | 'string' | 'heading' | 'code_fence' | 'list_marker' | 'inline_code';
}

export interface NanoLine {
  number: number;
  content: string;
  current: boolean;
  syntax: SyntaxHighlight[];
}

export interface NanoCursor {
  line: number;
  col: number;
}

export interface NanoEditor {
  type: 'nano_editor';
  filename: string;
  modified: boolean;
  lines: NanoLine[];
  cursor: NanoCursor;
  status: string;
  help: string;
}

export interface NanoEditorState {
  success: boolean;
  editor?: NanoEditor;
  error?: string;
}

export class Terminal {
  constructor();
  execute_command(input: string): CommandResponse;
  get_current_directory(): string;
  list_files(path?: string): ListFilesResponse;
  read_file(path: string): ReadFileResponse;
  write_file(path: string, content: string): WriteFileResponse;
  get_command_list(): string[];
  get_environment_variables(): Record<string, string>;
  set_environment_variable(key: string, value: string): void;
  is_nano_mode(): boolean;
  get_nano_filename(): string | null;
  process_nano_input(input: string): NanoResponse;
  get_nano_editor_state(): NanoEditorState;
}

export function get_assembly_template(template_type: 'basic' | 'hello' | 'loop'): string; 