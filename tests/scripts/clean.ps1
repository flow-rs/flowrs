Write-Host "Cleaning up Docker containers, networks, and output files..."

# Stop and remove all containers
docker-compose -f docker-compose.yml down

# Remove the output directory
$outputDir = "..\output"
if (Test-Path $outputDir) {
    Remove-Item -Recurse -Force -Path $outputDir
    Write-Host "Output directory cleaned."
}

# Remove logs
$logDir = ".\logs"
if (Test-Path $logDir) {
    Remove-Item -Recurse -Force -Path $logDir
    Write-Host "Log directory cleaned."
}

Write-Host "Clean-up completed successfully." -ForegroundColor Green
