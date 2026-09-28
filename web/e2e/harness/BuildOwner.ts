import { execFile, spawn } from 'node:child_process';

/** Own one command and its descendants; settle only after the child closes. */
export async function runOwnedCommand(
  command: string,
  args: string[],
  cwd: string,
  signal: AbortSignal,
  timeoutMs = 600_000,
): Promise<void> {
  signal.throwIfAborted();
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) {
    throw new Error('Invalid build command timeout');
  }
  await new Promise<void>((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
      env: process.env,
      stdio: 'inherit',
      detached: process.platform !== 'win32',
    });
    let failure: unknown;
    let treeTermination: Promise<void> | undefined;
    let escalation: ReturnType<typeof setTimeout> | undefined;
    const signalGroup = (kind: NodeJS.Signals) => {
      // Never retain a numeric process-group target after Node reaps its leader.
      if (!child.pid || child.exitCode !== null || child.signalCode !== null) return;
      try {
        process.kill(-child.pid, kind);
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== 'ESRCH') {
          failure ??= error;
          child.kill(kind);
        }
      }
    };
    const terminateTree = (): Promise<void> => {
      if (treeTermination) return treeTermination;
      treeTermination = new Promise<void>((done) => {
        if (!child.pid) return done();
        if (process.platform === 'win32') {
          // taskkill targets only this command's PID and descendants.
          execFile('taskkill', ['/PID', String(child.pid), '/T', '/F'], { timeout: 5_000 }, (error) => {
            if (error && child.exitCode === null && child.signalCode === null) {
              failure ??= error;
              child.kill('SIGKILL');
            }
            done();
          });
        } else {
          signalGroup('SIGTERM');
          escalation = setTimeout(() => signalGroup('SIGKILL'), 5_000);
          done();
        }
      });
      return treeTermination;
    };
    const abort = () => {
      failure ??= signal.reason ?? new Error('Build command cancelled');
      void terminateTree();
    };
    const timer = setTimeout(() => {
      failure ??= new Error(`${command} exceeded ${timeoutMs}ms build deadline`);
      void terminateTree();
    }, timeoutMs);
    signal.addEventListener('abort', abort, { once: true });
    child.once('error', (error) => { failure ??= error; });
    child.once('close', async (code) => {
      clearTimeout(timer);
      if (escalation) clearTimeout(escalation);
      signal.removeEventListener('abort', abort);
      if (treeTermination) await treeTermination;
      if (failure !== undefined) reject(failure);
      else if (code === 0) resolve();
      else reject(new Error(`${command} ${args.join(' ')} exited with code ${code}`));
    });
    if (signal.aborted) abort();
  });
}

type Build = {
  controller: AbortController;
  consumers: number;
  promise: Promise<void>;
};

/** Share a build only while it is running, with cancellation per consumer. */
export class SharedBuildOwner {
  private current?: Build;

  async run(signal: AbortSignal, build: (signal: AbortSignal) => Promise<void>): Promise<void> {
    signal.throwIfAborted();
    // A cancelled previous build must finish cleanup before its replacement.
    while (this.current?.controller.signal.aborted) {
      await this.current.promise.catch(() => {});
      signal.throwIfAborted();
    }
    if (!this.current) {
      const controller = new AbortController();
      const job: Build = { controller, consumers: 0, promise: Promise.resolve() };
      this.current = job;
      job.promise = Promise.resolve().then(() => build(controller.signal)).finally(() => {
        if (this.current === job) this.current = undefined;
      });
    }
    const job = this.current;
    job.consumers += 1;
    await new Promise<void>((resolve, reject) => {
      let finished = false;
      const finish = (error?: unknown) => {
        if (finished) return;
        finished = true;
        signal.removeEventListener('abort', abort);
        job.consumers -= 1;
        if (error === undefined) resolve(); else reject(error);
      };
      const abort = () => {
        const reason = signal.reason ?? new Error('Build consumer cancelled');
        if (job.consumers === 1) {
          job.controller.abort(reason);
          // The last consumer owns cleanup and waits for command close.
          void job.promise.then(() => finish(reason), () => finish(reason));
        } else {
          finish(reason);
        }
      };
      signal.addEventListener('abort', abort, { once: true });
      void job.promise.then(() => finish(signal.aborted ? signal.reason : undefined), (error) => finish(error));
      if (signal.aborted) abort();
    });
  }
}
