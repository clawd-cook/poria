function isFixtureMode(): boolean {
  return process.env.PORIA_PIPELINE_FIXTURE === "1";
}

export { isFixtureMode };
