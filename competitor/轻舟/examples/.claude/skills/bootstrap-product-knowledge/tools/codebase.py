#!/usr/bin/env python3
"""产品知识库代码库管理工具。

约定：
- 每个产品目录下有 codebase.json：
    {
      "product": "...",            # 产品目录名（英文 kebab-case，与目录同名）
      "name": "...",               # 产品中文名（可选，仅人读元信息）
      "description": "...",        # 产品一句话描述（可选，仅人读元信息）
      "repos": [
        { "git": "git@host:org/repo.git", "role": "职责", "branch": "分支名（可选）" }
      ]
    }
- 代码库统一缓存在 ~/.product-knowledge/codebase/<org>/<repo>（全局扁平共享，org+repo 全局唯一）；
  云端沙箱（环境变量 OPENCLI_SANDBOX_MODE）下代码由外部预置在 ~/workspace/、扁平按 repo 名，
  故改用 ~/workspace/<repo>——这是沙箱侧约定好的固定位置，不随知识库仓库放在哪而变。
  只有取路径这一步分叉，clone/path/list/mark/diff 其余行为两端一致。
- 分支为全局单缓存：同一 repo 只有一份目录，clone/pull 时 checkout 到 codebase.json 指定分支
  （不指定 branch 则用远端默认分支）。同一 repo 被多个产品以不同分支引用时会互相覆盖，实际少见。
- 路径权威归本程序：AI 不自己算路径，传 git 链接来问

增量更新基准（.knowledge-state.json）：
- 每个产品目录下有 <产品名>/.knowledge-state.json，记录「上次蒸馏时各 repo 的 commit SHA」。
- 这是增量更新的唯一基准：diff 拿它和当前 HEAD 比，算出哪些代码变了、进而判断哪些文档要刷新。
- 蒸馏完一轮后用 `mark` 写入基准；下次更新前用 `diff` 查变更。

子命令：
  clone <产品名>   读 codebase.json，对每个 repo 不存在则 clone、存在则 pull，并 checkout 指定分支；失败不阻断，结尾汇总
  path <git链接>   打印该 repo 的本地绝对路径；目录不存在则退出码非 0
  list <产品名>    输出 JSON 数组，每项 {git, role, branch, path}
  mark <产品名>    把各 repo 当前 HEAD 的 SHA 写进 .knowledge-state.json（蒸馏后调用，作为增量基准）
  diff <产品名>    对比 .knowledge-state.json 记录的 SHA 与当前 HEAD，输出各 repo 的变更文件清单
"""

import argparse
import json
import os
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path

CODEBASE_ROOT = Path.home() / ".product-knowledge" / "codebase"
# 产品目录（含 codebase.json）位于知识库仓库根，即运行本 skill 时的工作目录。
REPO_ROOT = Path.cwd()
# 云端沙箱：代码库由外部预置在 ~/workspace/，扁平按 repo 名，直接用，不再往缓存目录拉一份。
SANDBOX_CODEBASE_ROOT = Path.home() / "workspace"
STATE_FILENAME = ".knowledge-state.json"

# git@host:org/repo(.git)
_SSH_RE = re.compile(r"^git@[^:]+:(?P<org>[^/]+)/(?P<repo>.+?)(?:\.git)?$")


def is_sandbox():
    """是否跑在云端沙箱。语义对齐框架唯一真相源（CLI execution.ts / telemetry.detect_run_env）：
    truthy 仅 1/true/on/yes（trim 后大小写不敏感），别的值（含 0/false）一律当本地。
    每次调用实时读环境变量，不在 import 期定死。
    """
    raw = os.environ.get("OPENCLI_SANDBOX_MODE")
    return isinstance(raw, str) and raw.strip().lower() in ("1", "true", "on", "yes")


def parse_git_url(git_url):
    """从 git@host:org/repo.git 解析出 (org, repo)。"""
    m = _SSH_RE.match(git_url.strip())
    if not m:
        raise ValueError(f"无法解析 git 地址（期望 git@host:org/repo.git）：{git_url}")
    return m.group("org"), m.group("repo")


def local_path(git_url):
    """git 链接 -> 本地绝对路径：沙箱取 ~/workspace/<repo>，本地取缓存目录。"""
    org, repo = parse_git_url(git_url)
    if is_sandbox():
        return SANDBOX_CODEBASE_ROOT / repo
    return CODEBASE_ROOT / org / repo


def load_manifest(product):
    """读 <产品名>/codebase.json。"""
    manifest = REPO_ROOT / product / "codebase.json"
    if not manifest.exists():
        raise FileNotFoundError(f"找不到 {manifest}")
    with open(manifest, encoding="utf-8") as f:
        data = json.load(f)
    repos = data.get("repos", [])
    if not repos:
        raise ValueError(f"{manifest} 里没有 repos")
    return repos


def _run_git(args, cwd=None):
    return subprocess.run(
        ["git", *args],
        cwd=str(cwd) if cwd else None,
        capture_output=True,
        text=True,
    )


def _checkout_branch(dest, branch):
    """将 dest 仓库切到 branch 并更新。失败返回 stderr，成功返回 None。"""
    res = _run_git(["fetch", "origin", branch], cwd=dest)
    if res.returncode != 0:
        return res.stderr.strip()
    res = _run_git(["checkout", branch], cwd=dest)
    if res.returncode != 0:
        return res.stderr.strip()
    res = _run_git(["reset", "--hard", f"origin/{branch}"], cwd=dest)
    if res.returncode != 0:
        return res.stderr.strip()
    return None


def cmd_clone(product):
    repos = load_manifest(product)
    ok, failed = [], []
    for repo in repos:
        git_url = repo["git"]
        branch = repo.get("branch")
        try:
            dest = local_path(git_url)
        except ValueError as e:
            print(f"[跳过] {git_url}: {e}", file=sys.stderr)
            failed.append(git_url)
            continue

        label = f"{git_url}@{branch}" if branch else git_url

        if dest.exists():
            print(f"[pull] {label} -> {dest}")
            if not branch:
                res = _run_git(["pull", "--ff-only"], cwd=dest)
                if res.returncode != 0:
                    print(
                        f"[告警] {label} 更新失败，使用本地现有版本继续：\n{res.stderr.strip()}",
                        file=sys.stderr,
                    )
                    failed.append(git_url)
                    continue
        else:
            print(f"[clone] {label} -> {dest}")
            dest.parent.mkdir(parents=True, exist_ok=True)
            clone_args = ["clone"]
            if branch:
                clone_args += ["--branch", branch]
            clone_args += [git_url, str(dest)]
            res = _run_git(clone_args)
            if res.returncode != 0:
                print(
                    f"[告警] {label} 克隆失败：\n{res.stderr.strip()}",
                    file=sys.stderr,
                )
                failed.append(git_url)
                continue

        # 指定了分支时，确保切到该分支并对齐远端（覆盖此前别的产品可能留下的其他分支）
        if branch:
            err = _checkout_branch(dest, branch)
            if err:
                print(
                    f"[告警] {label} 切换分支失败，使用当前分支继续：\n{err}",
                    file=sys.stderr,
                )
                failed.append(git_url)
                continue

        ok.append(git_url)

    print(f"\n汇总：成功 {len(ok)} 个，失败 {len(failed)} 个")
    if failed:
        for g in failed:
            print(f"  失败: {g}")
    return 0


def cmd_path(git_url):
    dest = local_path(git_url)
    if not dest.exists():
        print(f"未找到本地代码库（请先 clone）：{dest}", file=sys.stderr)
        return 1
    print(str(dest))
    return 0


def cmd_list(product):
    repos = load_manifest(product)
    out = []
    for repo in repos:
        git_url = repo["git"]
        out.append(
            {
                "git": git_url,
                "role": repo.get("role", ""),
                "branch": repo.get("branch", ""),
                "path": str(local_path(git_url)),
            }
        )
    print(json.dumps(out, ensure_ascii=False, indent=2))
    return 0


def _state_path(product):
    return REPO_ROOT / product / STATE_FILENAME


def _head_sha(dest):
    """读 dest 仓库当前 HEAD 的完整 SHA；不可用返回 None。"""
    if not dest.exists():
        return None
    res = _run_git(["rev-parse", "HEAD"], cwd=dest)
    if res.returncode != 0:
        return None
    return res.stdout.strip()


def cmd_mark(product):
    """把各 repo 当前 HEAD 的 SHA 记为增量基准。"""
    repos = load_manifest(product)
    repos_state = {}
    missing = []
    for repo in repos:
        git_url = repo["git"]
        try:
            dest = local_path(git_url)
        except ValueError:
            missing.append(git_url)
            continue
        sha = _head_sha(dest)
        if sha is None:
            missing.append(git_url)
            continue
        repos_state[git_url] = {"branch": repo.get("branch", ""), "sha": sha}

    state = {"marked_at": datetime.now().isoformat(timespec="seconds"), "repos": repos_state}
    path = _state_path(product)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(state, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"已写入增量基准：{path}（{len(repos_state)} 个 repo）")
    if missing:
        print("以下 repo 未克隆或读不到 SHA，未记入基准：", file=sys.stderr)
        for g in missing:
            print(f"  {g}", file=sys.stderr)
    return 0


def cmd_diff(product):
    """对比 .knowledge-state.json 记录的 SHA 与当前 HEAD，输出各 repo 的变更文件清单。"""
    repos = load_manifest(product)
    path = _state_path(product)
    if not path.exists():
        print(
            f"没有增量基准 {path}：说明还没蒸馏过或没 mark。请先完成一轮蒸馏并 mark。",
            file=sys.stderr,
        )
        return 1
    with open(path, encoding="utf-8") as f:
        state = json.load(f)
    marked = state.get("repos", {})

    out = []
    for repo in repos:
        git_url = repo["git"]
        try:
            dest = local_path(git_url)
        except ValueError:
            continue
        cur = _head_sha(dest)
        base = marked.get(git_url, {}).get("sha")
        entry = {
            "git": git_url,
            "role": repo.get("role", ""),
            "branch": repo.get("branch", ""),
            "base_sha": base,
            "head_sha": cur,
        }
        if cur is None:
            entry["status"] = "未克隆"
            entry["files"] = []
        elif base is None:
            entry["status"] = "无基准（新增 repo，视为全量）"
            entry["files"] = []
        elif base == cur:
            entry["status"] = "无变更"
            entry["files"] = []
        else:
            res = _run_git(["diff", "--name-only", f"{base}..{cur}"], cwd=dest)
            if res.returncode != 0:
                entry["status"] = f"diff 失败（基准 SHA 可能已不在历史里）：{res.stderr.strip()}"
                entry["files"] = []
            else:
                files = [l for l in res.stdout.splitlines() if l.strip()]
                entry["status"] = f"变更 {len(files)} 个文件"
                entry["files"] = files
        out.append(entry)

    print(json.dumps(out, ensure_ascii=False, indent=2))
    return 0


def main():
    parser = argparse.ArgumentParser(description="产品知识库代码库管理工具")
    sub = parser.add_subparsers(dest="command", required=True)

    p_clone = sub.add_parser("clone", help="按产品 clone/pull 所有代码库")
    p_clone.add_argument("product", help="产品目录名")

    p_path = sub.add_parser("path", help="返回某 git 链接对应的本地路径")
    p_path.add_argument("git_url", help="git@host:org/repo.git")

    p_list = sub.add_parser("list", help="列出某产品所有 repo 的 git/role/path")
    p_list.add_argument("product", help="产品目录名")

    p_mark = sub.add_parser("mark", help="记录各 repo 当前 SHA 作为增量基准")
    p_mark.add_argument("product", help="产品目录名")

    p_diff = sub.add_parser("diff", help="对比基准 SHA 与当前 HEAD，输出变更文件清单")
    p_diff.add_argument("product", help="产品目录名")

    args = parser.parse_args()
    try:
        if args.command == "clone":
            return cmd_clone(args.product)
        if args.command == "path":
            return cmd_path(args.git_url)
        if args.command == "list":
            return cmd_list(args.product)
        if args.command == "mark":
            return cmd_mark(args.product)
        if args.command == "diff":
            return cmd_diff(args.product)
    except (FileNotFoundError, ValueError) as e:
        print(f"错误：{e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
