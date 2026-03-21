export type WatchSettings = {
  watchPath: string;
  destinationPath: string;
  startVersion?: string;
};

export type PromptPayload = {
  requestId: string;
  sourcePath: string;
  packageId: string;
  currentVersion: string;
  nextVersion: string;
  destinationPath: string;
  destinationFileName: string;
};

export type LocalPackageInfo = {
  packageId: string;
  latestVersion: string;
};
