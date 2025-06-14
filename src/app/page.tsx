"use client";

import Image from "next/image";
import React, { useRef, useState, useEffect } from "react";
import Logo from "./icon-white.png";
import { VfsEventProvider, useVfs } from "./VfsEventProvider";

// monospace font stack because designers don't understand developers
const FONT = "Monaco, Menlo, 'Ubuntu Mono', Consolas, source-code-pro, monospace";

// what kind of line this is in the terminal
interface TerminalLine {
  type: 'input' | 'output' | 'error';
  content: string;
}

// a line in the editor because we need to track line numbers
interface EditLine {
  number: number;
  content: string;
}

// editor state when someone actually wants to edit files
interface EditEditor {
  type: 'edit_editor';
  filename: string;
  modified: boolean;
  lines: EditLine[];
  total_lines: number;
  help: string;
}

// wasm terminal type - using any because typescript is annoying
type Terminal = any;

const Home = () => {
  // all the state we need to track
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const [current, setCurrent] = useState("");
  const [terminal, setTerminal] = useState<Terminal | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [currentDirectory, setCurrentDirectory] = useState("/");
  const [isEditMode, setIsEditMode] = useState(false);
  const [editEditor, setEditEditor] = useState<EditEditor | null>(null);
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState<number | null>(null);
  
  // refs for when we need to mess with the dom directly
  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const editContentRef = useRef<HTMLDivElement>(null);

  // vfs context because we need to read files somehow
  const vfs = useVfs();

  // load the wasm module when the component mounts
  useEffect(() => {
    const init = async () => {
      try {
        // wait for vfs to get its act together
        if (!vfs.isReady) {
          console.log('waiting for zenfs to be ready...');
          return;
        }

        // @ts-ignore
        const wasmModule = await import("./source/pkg/source");
        const { Terminal, default: init } = wasmModule;
        // typescript doesn't know about this function
        const set_async_result_callback = (wasmModule as any).set_async_result_callback;
        
        await init();
        const term = new Terminal();
        
        // try to initialize storage
        try {
          const storageResult = await term.init_with_storage();
          console.log('storage initialization:', storageResult);
          
          // load files from zenfs into rust vfs
          try {
            console.log('loading existing files from zenfs...');
            const allFiles = await vfs.getAllFiles();
            console.log(`found ${allFiles.length} files in zenfs:`, allFiles.map(f => f.path));
            
            if (allFiles.length > 0) {
              // convert to format rust expects
              const filesData = allFiles.map(file => ({
                path: file.path,
                content: Array.from(file.content)
              }));
              
              const loadResult = (term as any).load_filesystem_data(JSON.stringify(filesData));
              console.log('filesystem load result:', loadResult);
            } else {
            }
          } catch (loadError) {
            console.warn('failed to load existing files:', loadError);
            setLines([{ 
              type: 'output', 
              content: 'storage ready but failed to load existing files'
            }]);
          }
        } catch (error) {
          console.warn('storage initialization failed, continuing with in-memory only:', error);
          setLines([{ 
            type: 'output', 
            content: 'storage unavailable, using in-memory filesystem only'
          }]);
        }
        
        // set up callback to show async results in terminal
        const handleAsyncResult = (result: string) => {
          // split multi-line results and add each line
          const outputLines = result.split('\n');
          setLines(prev => [
            ...prev,
            ...outputLines.map((line: string) => ({ type: 'output' as const, content: line }))
          ]);
          
          // scroll to bottom because nobody wants to scroll manually
          setTimeout(() => {
            if (containerRef.current) {
              containerRef.current.scrollTop = containerRef.current.scrollHeight;
            }
          }, 0);
        };
        
        // register the callback with wasm if it exists
        if (set_async_result_callback) {
          set_async_result_callback(handleAsyncResult);
        }
        
        setTerminal(term);
        setIsLoading(false);
      } catch (error) {
        console.error('failed to initialize wasm module:', error);
        setLines([{ type: 'error', content: `Error: ${error}` }]);
        setIsLoading(false);
      }
    };
    init();
  }, [vfs.isReady]);

  // auto-scroll terminal to bottom because users expect it
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [lines, editEditor]);

  // focus input when terminal is ready
  useEffect(() => {
    if (!isLoading && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isLoading]);

  // scroll editor content when in edit mode
  useEffect(() => {
    if (isEditMode && editEditor && editContentRef.current) {
      // scroll to bottom of edit content
      editContentRef.current.scrollTop = editContentRef.current.scrollHeight;
    }
  }, [isEditMode, editEditor]);

  // handle input changes
  const handleInput = (e: React.ChangeEvent<HTMLInputElement>) => {
    setCurrent(e.target.value);
  };

  // execute command - handles both normal and edit modes
  const executeCommand = (command: string) => {
    if (!terminal || !command.trim()) return;

    // send to edit handler if we're in edit mode
    if (isEditMode) {
      handleEditCommand(command);
      return;
    }

    // clear is special because it's simple
    if (command.trim() === 'clear') {
      setLines([]);
      // scroll after clearing
      setTimeout(() => {
        if (containerRef.current) {
          containerRef.current.scrollTop = containerRef.current.scrollHeight;
        }
      }, 0);
      return;
    }

    // regular command execution
    setLines(prev => [...prev, { type: 'input', content: command }]);

    // check if this is an async command that takes forever
    const isAsyncCommand = command.trim().startsWith('ping ') || command.trim().startsWith('curl ');

    try {
      // call wasm terminal
      const response = terminal.execute_command(command);
      
      if (response.success) {
        if (response.output) {
          // check if switching to edit mode
          if (command.startsWith('edit ')) {
            try {
              // try parsing as editor data
              const editorData = JSON.parse(response.output);
              if (editorData.type === 'edit_editor') {
                setIsEditMode(true);
                setEditEditor(editorData);
                return; // don't show json output
              }
            } catch (e) {
              // not json, show as regular output
              console.log('failed to parse edit output as json, treating as regular output');
            }
          }
          // handle clear marker from backend
          if (response.output.trim() === '__CLEAR_SCREEN__') {
            setLines([]);
            setTimeout(() => {
              if (containerRef.current) {
                containerRef.current.scrollTop = containerRef.current.scrollHeight;
              }
            }, 0);
            return;
          }
          // handle multi-line output
          const outputLines = response.output.split('\n');
          setLines(prev => [
            ...prev,
            ...outputLines.map((line: string) => ({ type: 'output' as const, content: line }))
          ]);
          
          // add note for async commands
          if (isAsyncCommand && response.output.includes('Results will appear in terminal as they arrive')) {
            setLines(prev => [
              ...prev,
              { type: 'output', content: 'tip: real network requests are running in the background. results will stream in below.' }
            ]);
          }
        }
      } else {
        // check for cors/network errors and add guidance
        let errorMsg = response.output;
        if (errorMsg.match(/CORS|network error|host unreachable|Failed to fetch|NetworkError|TypeError/)) {
          errorMsg +=
            '\n[frontend] note: most public sites block browser requests due to cors, so this is probably not your fault. try a cors-friendly test endpoint like https://httpbin.org/get';
        }
        setLines(prev => [...prev, { type: 'error', content: errorMsg }]);
      }

      // keep current directory in sync
      setCurrentDirectory(terminal.get_current_directory());
      // always scroll to bottom after running a command
      setTimeout(() => {
        if (containerRef.current) {
          containerRef.current.scrollTop = containerRef.current.scrollHeight;
        }
      }, 0);
    } catch (error) {
      console.error('command execution error:', error);
      setLines(prev => [...prev, { type: 'error', content: `Error: ${error}` }]);
      setTimeout(() => {
        if (containerRef.current) {
          containerRef.current.scrollTop = containerRef.current.scrollHeight;
        }
      }, 0);
    }
  };

  // handle commands while in edit mode
  const handleEditCommand = (input: string) => {
    if (!terminal) return;

    // don't log edit mode input to terminal history
    // setLines(prev => [...prev, { type: 'input', content: input }]);

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
          setHistoryIndex(null);
          // scroll to bottom when returning to terminal mode
          setTimeout(() => {
            if (containerRef.current) {
              containerRef.current.scrollTop = containerRef.current.scrollHeight;
            }
          }, 0);
        }
      } else {
        setLines(prev => [...prev, { type: 'error', content: response.output }]);
      }
    } catch (error) {
      console.error('edit command error:', error);
      setLines(prev => [...prev, { type: 'error', content: `Error: ${error}` }]);
    }
  };

  // handle keyboard shortcuts and history
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    // ctrl+l: clear screen like every other terminal
    if (e.ctrlKey && e.key === 'l') {
      e.preventDefault();
      executeCommand('clear');
      setCurrent('');
      return;
    }
    // up arrow: previous command from history
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (history.length === 0) return;
      setHistoryIndex(prev => {
        let idx = prev === null ? history.length - 1 : prev - 1;
        if (idx < 0) idx = 0;
        setCurrent(history[idx]);
        return idx;
      });
      return;
    }
    // down arrow: next command from history
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (history.length === 0) return;
      setHistoryIndex(prev => {
        let idx = prev === null ? history.length - 1 : prev + 1;
        if (idx >= history.length) {
          setCurrent('');
          return null;
        }
        setCurrent(history[idx]);
        return idx;
      });
      return;
    }
    if (e.key === 'Enter') {
      executeCommand(current);
      setHistory(prev => (current && (prev.length === 0 || prev[prev.length - 1] !== current)) ? [...prev, current] : prev);
      setHistoryIndex(null);
      setCurrent('');
    }
  };

  // execute predefined commands (for quick actions)
  const handleQuickCommand = (command: string) => {
    if (!terminal) return;

    // clear command is special
    if (command.trim() === 'clear') {
      setLines([]);
      terminal.execute_command('clear');
      return;
    }

    // regular command execution
    terminal.execute_command(command);
  };

  // test function to manually trigger vfs events
  const testVfsEvents = () => {
    if (!terminal) return;
    
    console.log('[test] testing vfs event system...');
    
    // test the rust event emitter
    try {
      const result = (terminal as any).test_emit_event();
      console.log('[test] rust test event result:', result);
    } catch (error) {
      console.error('[test] failed to call rust test function:', error);
    }
  };

  // handle drag and drop file upload
  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();

    const files = Array.from(e.dataTransfer.files);
    if (files.length === 0) return;

    // send each file to the terminal
    files.forEach(file => {
      // use webkitRelativePath if available, otherwise just use the file name
      const filePath = file.webkitRelativePath || file.name;
      if (filePath) {
        setLines(prev => [...prev, { type: 'input', content: `upload ${filePath}` }]);
        terminal.execute_command(`upload ${filePath}`);
      }
    });
  };

  // handle file input change (for manual file selection)
  const handleFileInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    if (files.length === 0) return;

    // send each file to the terminal
    files.forEach(file => {
      // use webkitRelativePath if available, otherwise just use the file name
      const filePath = file.webkitRelativePath || file.name;
      if (filePath) {
        setLines(prev => [...prev, { type: 'input', content: `upload ${filePath}` }]);
        terminal.execute_command(`upload ${filePath}`);
      }
    });
  };

  // handle drag over event (to allow drops)
  const handleDragOver = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
  };

  // render terminal line based on type
  const renderLine = (line: TerminalLine, index: number) => {
    switch (line.type) {
      case 'input':
        return <div key={index} className="text-green-400">{`> ${line.content}`}</div>;
      case 'output':
        return <div key={index} className="text-white">{line.content}</div>;
      case 'error':
        return <div key={index} className="text-red-400">{line.content}</div>;
      default:
        return null;
    }
  };

  // render editor line with line number
  const renderEditLine = (line: EditLine, index: number) => {
    return <div key={index} className="flex items-center">
      <div className="text-gray-500 pr-2">{`#${line.number}`}</div>
      <div className="flex-1">
        <div className="bg-gray-800 p-2 rounded-md">
          <pre className="text-white whitespace-pre-wrap break-words">{line.content}</pre>
        </div>
      </div>
    </div>;
  };

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

      {/* loader overlay */}
      {isLoading && (
        <div className="fixed inset-0 flex items-center justify-center bg-black bg-opacity-75 z-50">
          <div className="text-white">Loading...</div>
        </div>
      )}


      {/* drag and drop area */}
      <div
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        className="fixed inset-0 z-40"
      />
    </div>
  );
};

// wrap home in vfseventprovider
const Page = () => (
  <VfsEventProvider>
    <Home />
  </VfsEventProvider>
);

export default Page;
