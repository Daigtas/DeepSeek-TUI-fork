# Sub-Agent Chunking Strategy

## Problem
Sub-agents time out at 120s API limit when reading large files (>400 lines).

## Solution: Chunked Reading

### For Sub-Agents (modify your approach)
Instead of `read_file("large-file.tsx")` which loads the ENTIRE file:
```typescript
// ❌ TIMES OUT — reads entire 3000-line schema
read_file("prisma/schema.prisma")

// ✅ CHUNKED — read in 400-line sections
read_file("prisma/schema.prisma", { limit: 400 })           // lines 1-400
read_file("prisma/schema.prisma", { limit: 400, offset: 400 })  // lines 401-800
read_file("prisma/schema.prisma", { limit: 400, offset: 800 })  // lines 801-1200
```

### For Orchestrator (me)
Use `rlm` tool for files > 400 lines:
```typescript
// rlm loads the file into a Python REPL, processes in chunks, returns synthesized answer
rlm({
  task: "Extract all Prisma models related to workflows",
  file_path: "prisma/schema.prisma",
  max_depth: 1
})
```

### File Chunking Rules
| File Size | Strategy | 
|-----------|----------|
| < 400 lines | Read directly |
| 400-800 lines | Read in 2 chunks |
| 800-2000 lines | Use `rlm` tool |
| > 2000 lines | Use `rlm` tool with `max_depth: 2` |

### Sub-Agent Timeout Config
Added to `~/.deepseek/config.toml`:
```toml
[subagents]
timeout_ms = 600000    # 10 minutes (was 120s)
max_concurrent = 10
chunk_large_files = true
chunk_size = 400       # split at 400 lines
```

## Agent Best Practices

1. **Estimate before reading**: Check file size with `wc -l` before reading
2. **Grep first, read targeted sections**: Use `grep_files` to find relevant sections, then read only those
3. **Use `exec_shell` for counts**: `wc -l file.tsx` to check size before reading
4. **Parallel reads for large codebases**: Read multiple 400-line chunks in parallel
5. **Use `rlm` for whole-file analysis**: When you need to analyze entire files
