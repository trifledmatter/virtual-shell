"use client";

import Image from "next/image";
import React, { useRef, useState, useEffect } from "react";
import Logo from "./icon-white.png";

// custom monospace font stack - fallback to system fonts if needed
const FONT = "Monaco, Menlo, 'Ubuntu Mono', Consolas, source-code-pro, monospace";

// terminal output line types
interface TerminalLine {
  type: 'input' | 'output' | 'error';
  content: string;
}

// representation of a single line in editor
interface EditLine {
  number: number;
  content: string;
}

// editor state when in edit mode
interface EditEditor {
  type: 'edit_editor';
  filename: string;
  modified: boolean;
  lines: EditLine[];
  total_lines: number;
  help: string;
}

// wasm terminal type - using any to avoid typing headaches
type Terminal = any;

const Home = () => {
  // state management
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const [current, setCurrent] = useState("");
  const [terminal, setTerminal] = useState<Terminal | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [currentDirectory, setCurrentDirectory] = useState("/");
  const [isEditMode, setIsEditMode] = useState(false);
  const [editEditor, setEditEditor] = useState<EditEditor | null>(null);
  
  // refs for dom manipulation
  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const editContentRef = useRef<HTMLDivElement>(null);

  // load wasm module on component mount
  useEffect(() => {
    const init = async () => {
      try {
        const { Terminal, default: init } = await import("./source/pkg/source");
        await init();
        const term = new Terminal();
        setTerminal(term);
        setIsLoading(false);
      } catch (error) {
        console.error('Failed to initialize WASM module:', error);
        setLines([{ type: 'error', content: `Error: ${error}` }]);
        setIsLoading(false);
      }
    };
    init();
  }, []);

  // auto-scroll terminal to bottom whenever content changes
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [lines, editEditor]);

  // auto-focus input when terminal is ready
  useEffect(() => {
    if (!isLoading && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isLoading]);

  // auto-scroll editor content when in edit mode
  useEffect(() => {
    if (isEditMode && editEditor && editContentRef.current) {
      // scroll to bottom of edit content
      editContentRef.current.scrollTop = editContentRef.current.scrollHeight;
    }
  }, [isEditMode, editEditor]);

  // handle input field changes
  const handleInput = (e: React.ChangeEvent<HTMLInputElement>) => {
    setCurrent(e.target.value);
  };

  // execute terminal command - handles both normal and edit modes
  const executeCommand = (command: string) => {
    if (!terminal || !command.trim()) return;

    // route to edit mode handler if active
    if (isEditMode) {
      handleEditCommand(command);
      return;
    }

    // special handling for clear command
    if (command.trim() === 'clear') {
      setLines([]);
      return;
    }

    // regular command execution flow
    setLines(prev => [...prev, { type: 'input', content: command }]);

    try {
      // call wasm terminal
      const response = terminal.execute_command(command);
      
      if (response.success) {
        if (response.output) {
          // check if switching to edit mode
          if (command.startsWith('edit ')) {
            try {
              // try parsing output as editor data
              const editorData = JSON.parse(response.output);
              if (editorData.type === 'edit_editor') {
                setIsEditMode(true);
                setEditEditor(editorData);
                return; // skip showing json output
              }
            } catch (e) {
              // fall back to regular output on parse fail
              console.log('Failed to parse edit output as JSON, treating as regular output');
            }
          }
          // handle clear marker from backend
          if (response.output.trim() === '__CLEAR_SCREEN__') {
            setLines([]);
            return;
          }
          // handle multi-line output
          const outputLines = response.output.split('\n');
          setLines(prev => [
            ...prev,
            ...outputLines.map((line: string) => ({ type: 'output' as const, content: line }))
          ]);
        }
      } else {
        setLines(prev => [...prev, { type: 'error', content: response.output }]);
      }

      // keep current directory state in sync
      setCurrentDirectory(terminal.get_current_directory());
    } catch (error) {
      console.error('Command execution error:', error);
      setLines(prev => [...prev, { type: 'error', content: `Error: ${error}` }]);
    }
  };

  // handle commands while in edit mode
  const handleEditCommand = (input: string) => {
    if (!terminal) return;

    // log input to terminal history
    setLines(prev => [...prev, { type: 'input', content: input }]);

    try {
      // pass to wasm with edit_input prefix
      const response = terminal.execute_command(`edit_input ${input}`);
      
      if (response.success) {
        if (response.output) {
          try {
            // update editor state if response contains editor data
            const editorData = JSON.parse(response.output);
            if (editorData.type === 'edit_editor') {
              setEditEditor(editorData);
              return;
            }
          } catch (e) {
            // not editor data, show as regular output
          }
          
          // display regular command output
          const outputLines = response.output.split('\n');
          setLines(prev => [
            ...prev,
            ...outputLines.map((line: string) => ({ type: 'output' as const, content: line }))
          ]);
        }
        
        // check for exit commands
        if (input === ':q' || input === ':wq') {
          setIsEditMode(false);
          setEditEditor(null);
        }
      } else {
        setLines(prev => [...prev, { type: 'error', content: response.output }]);
      }
    } catch (error) {
      console.error('Edit command error:', error);
      setLines(prev => [...prev, { type: 'error', content: `Error: ${error}` }]);
    }
  };

  // handle enter key for command execution
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    // Ctrl+L: clear screen
    if (e.ctrlKey && e.key === 'l') {
      e.preventDefault();
      executeCommand('clear');
      setCurrent('');
      return;
    }
    if (e.key === 'Enter') {
      executeCommand(current);
      setCurrent('');
    }
  };

  // execute predefined commands (for quick actions)
  const handleQuickCommand = (command: string) => {
    if (isEditMode) return;
    setCurrent(command);
    executeCommand(command);
    setCurrent('');
  };


  // loading screen
  if (isLoading) {
    return (
      <div className="w-screen h-screen bg-black flex items-center justify-center">
        <div className="text-white text-xl">Loading terminal...</div>
      </div>
    );
  }

  // main terminal ui
  return (
    <div
      ref={containerRef}
      className="w-screen h-screen bg-black flex flex-col overflow-hidden"
      style={{ fontFamily: FONT }}
      onClick={() => inputRef.current?.focus()}
    >

      {/* terminal content area */}
      <div className="flex-1 overflow-y-auto p-4 hide-scrollbar">
        <div className="text-white font-mono text-lg w-full">
          {/* editor view when in edit mode */}
          {isEditMode && editEditor ? (
            <div className="mb-4">
              {/* file editor ui */}
              <div className="border border-gray-700 rounded p-4 mb-4">
                <div className="text-gray-300 mb-2">
                  <span className="text-cyan-400">{editEditor.filename}</span>
                  <span className="text-gray-500 ml-4">({editEditor.total_lines} lines)</span>
                </div>
                <div 
                  ref={editContentRef}
                  className="bg-black border border-gray-600 p-2 max-h-96 overflow-y-auto hide-scrollbar"
                >
                  {editEditor.lines.length === 0 ? (
                    <div className="text-gray-500 italic">Empty file</div>
                  ) : (
                    editEditor.lines.map((line, idx) => (
                      <div key={idx} className="flex">
                        <span className="text-gray-500 text-sm w-8 text-right mr-2 flex-shrink-0">
                          {line.number}
                        </span>
                        <span className="flex-1 font-mono text-sm whitespace-pre">
                          {line.content || <span className="text-gray-600">{"<empty>"}</span>}
                        </span>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </div>
          ) : (
            /* normal terminal output history */
            lines.map((line, idx) => (
              <div key={idx} className="flex w-full items-start mb-1">
                {line.type === 'input' ? (
                  <>
                    <span className="text-purple-400 flex-shrink-0 flex items-center justify-center h-full">
                      <Image alt="trifledmatter-logo" width={32} height={32} src={Logo} className="w-4 h-4" />
                      &nbsp;
                    </span>
                    <span className="text-cyan-400 flex-shrink-0 mr-2 font-bold">
                      [virt::core] ➤
                    </span>
                    <span className="flex-1 break-all">{line.content}</span>
                  </>
                ) : line.type === 'error' ? (
                  <span className="flex-1 break-all text-red-400 ml-7">{line.content}</span>
                ) : (
                  <span className="flex-1 break-all text-gray-300 ml-7 whitespace-pre-wrap">{line.content}</span>
                )}
              </div>
            ))
          )}
          
          {/* input prompt line */}
          <div className="flex w-full items-center">
            <span className="text-purple-400 flex-shrink-0 flex items-center justify-center">
              <Image alt="trifledmatter-logo" width={32} height={32} src={Logo} className="w-4 h-4" />
              &nbsp;
            </span>
            <span className="text-cyan-400 flex-shrink-0 mr-2 font-bold">
              {isEditMode ? '[edit]' : '[virt::core]'} ➤
            </span>
            <input
              ref={inputRef}
              className="bg-transparent outline-none border-none text-white font-mono text-lg flex-1 caret-cyan-400"
              style={{ fontFamily: FONT }}
              value={current}
              onChange={handleInput}
              onKeyDown={handleKeyDown}
              autoFocus
              spellCheck={false}
              autoComplete="off"
              autoCorrect="off"
              autoCapitalize="off"
              placeholder={isEditMode ? "Enter line number and content (e.g., '5 push 10', '* ;') or :q, :w, :wq" : ""}
            />
          </div>
        </div>
      </div>
    </div>
  );
};

export default Home;
