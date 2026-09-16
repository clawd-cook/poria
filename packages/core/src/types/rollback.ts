export interface RollbackCommand {
  type: "delete_branch" | "close_mr" | "revert_commit" | "remove_worktree" | "revert_mr";
  params: Record<string, string>;
}

export interface RollbackInstruction {
  stageIndex: number;
  commands: RollbackCommand[];
}
