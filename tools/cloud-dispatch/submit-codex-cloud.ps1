param(
  [Parameter(Mandatory = $true)]
  [string] $EnvId,

  [Parameter(Mandatory = $true)]
  [string] $PromptFile,

  [string] $Branch = "main",

  [int] $Attempts = 1
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $PromptFile)) {
  throw "Prompt file not found: $PromptFile"
}

$prompt = Get-Content -Raw -LiteralPath $PromptFile
if ([string]::IsNullOrWhiteSpace($prompt)) {
  throw "Prompt file is empty: $PromptFile"
}

codex cloud exec --env $EnvId --branch $Branch --attempts $Attempts -- $prompt
