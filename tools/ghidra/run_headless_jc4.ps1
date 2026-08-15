# Headless Ghidra: import + auto-analyze JustCause4.exe, then export all decompiled functions.
# Usage (from the repo root):
#   powershell -File tools/ghidra/run_headless_jc4.ps1 -MaxMem 12G -MaxCpu 6 -AnalysisTimeout 0
#
# -MaxMem           JVM heap for the headless run (GHIDRA_HEADLESS_MAXMEM). 268 MB exe on 16 GB RAM is
#                   tight; leave a few GB for the OS. Default 12G.
# -MaxCpu           analysis threads. Default 6 (of 8) so the box stays usable.
# -AnalysisTimeout  seconds per-file analysis cap; 0 = no cap (large binary may need it). Default 0.
param(
  [string]$MaxMem = "12G",
  [int]$MaxCpu = 6,
  [int]$AnalysisTimeout = 0
)

$ErrorActionPreference = "Stop"
$Repo    = "C:\Users\Shadow\Desktop\notes-on-reversing-jc4"
$Ghidra  = "C:\Users\Shadow\Desktop\notes-on-the-released-game\tools\ghidra_12.1_PUBLIC"
$Jdk     = "C:\Users\Shadow\Desktop\notes-on-the-released-game\tools\jdk21\jdk-21.0.11+10"
$Exe     = "C:\Program Files (x86)\Steam\steamapps\common\Just Cause 4\JustCause4.exe"
$ProjDir = "$Repo\output\ghidra_project"
$ScriptDir = "$Repo\tools\ghidra"

$env:JAVA_HOME = $Jdk
$env:GHIDRA_HEADLESS_MAXMEM = $MaxMem
New-Item -ItemType Directory -Force -Path $ProjDir | Out-Null

# analyzeHeadless.bat (cmd) can't parse the parentheses in "Program Files (x86)" — use the 8.3 short path.
$ExeImport = (New-Object -ComObject Scripting.FileSystemObject).GetFile($Exe).ShortPath

$headless = "$Ghidra\support\analyzeHeadless.bat"
$args = @(
  $ProjDir, "JC4",
  "-import", $ExeImport,
  "-scriptPath", $ScriptDir,
  "-postScript", "DecompileJC4.java",
  "-max-cpu", "$MaxCpu"
)
if ($AnalysisTimeout -gt 0) { $args += @("-analysisTimeoutPerFile", "$AnalysisTimeout") }

Write-Output "JAVA_HOME=$env:JAVA_HOME"
Write-Output "GHIDRA_HEADLESS_MAXMEM=$env:GHIDRA_HEADLESS_MAXMEM  max-cpu=$MaxCpu"
Write-Output "importing $Exe -> project $ProjDir\JC4"
& $headless @args
Write-Output "analyzeHeadless exit code: $LASTEXITCODE"
