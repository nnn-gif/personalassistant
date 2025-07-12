declare global {
  interface Window {
    __TAURI__: {
      core: {
        invoke: (cmd: string, args?: any) => Promise<any>
        convertFileSrc: (filePath: string, protocol?: string) => string
      }
    }
  }
}

export {}