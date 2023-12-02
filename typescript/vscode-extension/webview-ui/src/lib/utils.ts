import { Mutex, type MutexInterface } from "async-mutex";

// Usage: await sleep(10)
// https://stackoverflow.com/questions/951021/what-is-the-javascript-version-of-sleep
export function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export class WriterReadersMutex {
  private reader_mutex = new Mutex();
  private writer_mutex = new Mutex();
  private n_reader = 0;
  private writer_releaser: MutexInterface.Releaser | undefined;

  // TODO Log something with sentry whenever there is an error detected here.
  public reader_acquire(): void {
    this._reader_acquire().catch((err) => console.log(err));
  }

  public reader_release(): void {
    this._reader_release().catch((err) => console.log(err));
  }

  public writer_acquire(): void {
    this._writer_acquire().catch((err) => console.log(err));
  }

  public writer_release(): void {
    if (this.writer_releaser) {
      const tmp_releaser = this.writer_releaser;
      this.writer_releaser = undefined;
      tmp_releaser();
    }
  }

  private async _reader_acquire(): Promise<void> {
    const reader_releaser = await this.reader_mutex.acquire();
    this.n_reader += 1;
    if (this.n_reader == 1) {
      this.writer_releaser = await this.writer_mutex.acquire();
    }
    reader_releaser();
  }

  private async _reader_release(): Promise<void> {
    const reader_releaser = await this.reader_mutex.acquire();
    this.n_reader -= 1;
    if (this.n_reader == 0) {
      this.writer_releaser?.();
    }
    reader_releaser();
  }

  private async _writer_acquire(): Promise<void> {
    this.writer_releaser = await this.writer_mutex.acquire();
  }
}
