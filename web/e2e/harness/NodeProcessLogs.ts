import { open, type FileHandle } from 'node:fs/promises';
import type { ChildProcess } from 'node:child_process';
import { pipeline } from 'node:stream/promises';

/** Pipes apply backpressure and this owner joins writes and closes every handle. */
export class NodeProcessLogs {
  private drains: Promise<void>[] = [];
  private failure?: unknown;
  private closing?: Promise<void>;

  private constructor(private handles: FileHandle[]) {}

  static async open(stdoutPath: string, stderrPath: string): Promise<NodeProcessLogs> {
    const stdout = await open(stdoutPath, 'w');
    try {
      const stderr = await open(stderrPath, 'w');
      return new NodeProcessLogs([stdout, stderr]);
    } catch (error) {
      await stdout.close();
      throw error;
    }
  }

  connect(child: ChildProcess): void {
    if (this.closing || this.drains.length) throw new Error('Node logs already connected or closing');
    for (const [index, source] of [child.stdout, child.stderr].entries()) {
      if (!source) continue;
      this.drains.push(pipeline(source, this.handles[index].createWriteStream())
        .catch((error) => { this.failure ??= error; }));
    }
  }

  close(): Promise<void> {
    if (!this.closing) {
      this.closing = (async () => {
        await Promise.all(this.drains);
        const results = await Promise.allSettled(this.handles.map((handle) => handle.close()));
        for (const result of results) {
          if (result.status === 'rejected') this.failure ??= result.reason;
        }
        if (this.failure !== undefined) throw this.failure;
      })();
    }
    return this.closing;
  }
}
