// Messages exchanged between the views and the extension.

/* eslint-disable @typescript-eslint/no-unsafe-assignment */

// **********************************
// Message from Extension to Views
// **********************************
export class ViewMessages {
  // Must be defined by all derived classes to match the class Name.
  name: string;
  sender: string;

  constructor(name: string, sender: string) {
    this.name = name; // Class name of derived class.
    this.sender = sender; // Unique identifier (see WEBVIEW_* in Consts.ts).
  }
}

export class UpdateVersions extends ViewMessages {
  workdirIdx: number;
  setupIssue: string | undefined; // Info for user when Suibase backend not available.
  json: any;

  constructor(sender: string, workdirIdx: number, json: any) {
    super("UpdateVersions", sender);
    this.workdirIdx = workdirIdx;
    this.setupIssue = undefined;
    this.json = json;
  }

  setSetupIssue(issue: string) {
    if (issue === "") {
      this.setupIssue = undefined;
    } else {
      this.setupIssue = issue;
    }
  }
}

export class UpdateWorkdirStatus extends ViewMessages {
  workdirIdx: number;
  json: any;

  constructor(sender: string, workdirIdx: number, json: any) {
    super("UpdateWorkdirStatus", sender);
    this.workdirIdx = workdirIdx;
    this.json = json;
  }
}

export class UpdateWorkdirPackages extends ViewMessages {
  workdirIdx: number;
  json: any;

  constructor(sender: string, workdirIdx: number, json: any) {
    super("UpdateWorkdirPackages", sender);
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
  constructor(sender: string, workdirIdx: number, command: string) {
    super("WorkdirCommand", sender);
    this.workdirIdx = workdirIdx;
    this.command = command;
  }
}

export class SuiCommand extends ViewMessages {
  workdirIdx: number;
  command: string; // e.g. "client switch --address some_alias"

  // Just request the backend to run the specified CLI command for a workdir.
  constructor(sender: string, workdirIdx: number, command: string) {
    super("SuiCommand", sender);
    this.workdirIdx = workdirIdx;
    this.command = command;
  }
}

export class InitView extends ViewMessages {
  // Request the extension to send all data commonly needed by a view
  // (sync with the backend as needed).
  constructor(sender: string) {
    super("InitView", sender);
  }
}

export class ForceVersionsRefresh extends ViewMessages {
  // Request the extension to send the latest Versions information.
  // It is assumed the view will further request what is needed.
  constructor(sender: string) {
    super("ForceVersionsRefresh", sender);
  }
}

// Requests from the view to update the status for a specific workdir.
// The extension will eventually reply with an UpdateWorkdirStatus.
// (will sync with the backend as needed).
export class RequestWorkdirStatus extends ViewMessages {
  workdirIdx: number;
  methodUuid: string;
  dataUuid: string;


  constructor(sender: string, workdirIdx: number, methodUuid: string, dataUuid: string) {
    super("RequestWorkdirStatus", sender);
    this.workdirIdx = workdirIdx;
    this.methodUuid = methodUuid;
    this.dataUuid = dataUuid;
  }
}

export class RequestWorkdirPackages extends ViewMessages {
  workdirIdx: number;
  methodUuid: string;
  dataUuid: string;

  // Request the extension to send all data commonly needed by a view
  // (sync with the backend as needed).
  constructor(sender: string, workdirIdx: number, methodUuid: string, dataUuid: string) {
    super("RequestWorkdirPackages", sender);
    this.workdirIdx = workdirIdx;
    this.methodUuid = methodUuid;
    this.dataUuid = dataUuid;
  }
}

