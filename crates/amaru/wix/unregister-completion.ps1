[CmdletBinding()]
param()

$startMarker = '# >>> amaru completion >>>'
$endMarker = '# <<< amaru completion <<<'
$pattern = "(?ms)^$([regex]::Escape($startMarker))\r?\n.*?^$([regex]::Escape($endMarker))\r?\n?"
$profilePaths = New-Object System.Collections.Generic.List[string]
$profilePaths.Add((Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\profile.ps1'))

$powerShellRoot = Join-Path $env:ProgramFiles 'PowerShell'
if (Test-Path -LiteralPath $powerShellRoot) {
    Get-ChildItem -LiteralPath $powerShellRoot -Directory | ForEach-Object {
        $profilePaths.Add((Join-Path $_.FullName 'profile.ps1'))
    }
}

$profilePaths |
    Select-Object -Unique |
    ForEach-Object {
        $profilePath = $_
        if (Test-Path -LiteralPath $profilePath) {
            $currentContent = Get-Content -LiteralPath $profilePath -Raw
            $cleanedContent = [regex]::Replace($currentContent, $pattern, '').TrimEnd()

            if ([string]::IsNullOrWhiteSpace($cleanedContent)) {
                Remove-Item -LiteralPath $profilePath -Force
            } else {
                Set-Content -LiteralPath $profilePath -Value ($cleanedContent + [Environment]::NewLine) -Encoding UTF8
            }
        }
    }
