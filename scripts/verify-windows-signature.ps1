param([Parameter(Mandatory=$true)][string]$Path)
$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($env:EXPECTED_SUBJECT)) { throw 'Expected signer subject is not configured.' }
$sig = Get-AuthenticodeSignature -LiteralPath $Path
if ($sig.Status -ne 'Valid') { throw "Invalid Authenticode signature: $($sig.Status)" }
if ($sig.SignerCertificate.Subject -cne $env:EXPECTED_SUBJECT) { throw 'Unexpected signing publisher.' }
if ($null -eq $sig.TimeStamperCertificate) { throw 'No countersigning timestamp certificate.' }
Write-Output 'Signature chain, expected publisher, and timestamp verified.'
