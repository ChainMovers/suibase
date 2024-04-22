// Messages exchanged between the views and the extension.

// **********************************
// Message from Extension to Views
// **********************************
export class ViewMessages {
  // Must be defined by all derived classes to match the class Name.
  name: string;

  constructor(name: string) {
    this.name = name;
  }
}

export class UpdateVersions extends ViewMessages {
  workdirIdx: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  json: any;

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  constructor(workdirIdx: number, json: any) {
    super("UpdateVersions");
    this.workdirIdx = workdirIdx;
    this.json = json;
  }
}

export class UpdateWorkdirStatus extends ViewMessages {
  workdirIdx: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  json: any;

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  constructor(workdirIdx: number, json: any) {
    super("UpdateWorkdirStatus");
    this.workdirIdx = workdirIdx;
    this.json = json;
  }
}

// **********************************
// Message from Views to Extension
// **********************************
export class WorkdirCommand extends ViewMessages {
  workdirIdx: number;
  command: string; // e.g. "start", "stop"

  // Just request the backend to run the specified CLI command for a workdir.
  constructor(workdirIdx: number, command: string) {
    super("WorkdirCommand");
    this.workdirIdx = workdirIdx;
    this.command = command;
  }
}

export class SuiCommand extends ViewMessages {
  workdirIdx: number;
  command: string; // e.g. "client switch --address some_alias"

  // Just request the backend to run the specified CLI command for a workdir.
  constructor(workdirIdx: number, command: string) {
    super("SuiCommand");
    this.workdirIdx = workdirIdx;
    this.command = command;
  }
}

export class InitView extends ViewMessages {
  // Request the extension to send all data commonly needed by a view
  // (sync with the backend as needed).
  constructor() {
    super("InitView");
  }
}

export class ForceVersionsRefresh extends ViewMessages {
  // Request the extension to send the latest Versions information.
  // It is assumed the view will further request what is needed.
  constructor() {
    super("ForceVersionsRefresh");
  }
}
