# Start the distributed test environment
Write-Host "Starting distributed test environment..."
docker-compose -f .\docker\docker-compose.yml up orchestrator nodert1 nodert2 --abort-on-container-exit

# Collect logs from the containers
Write-Host "Collecting logs..."
$logDir = ".\logs"
if (-Not (Test-Path $logDir)) {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}

docker logs orchestrator > "$logDir\orchestrator.log"
docker logs nodert1 > "$logDir\nodert1.log"
docker logs nodert2 > "$logDir\nodert2.log"

Write-Host "Logs collected in $logDir."

# Tear down the environment
Write-Host "Tearing down environment..."
docker-compose -f .\docker\docker-compose.yml down
Write-Host "Environment torn down."