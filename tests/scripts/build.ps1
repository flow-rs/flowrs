# Set the output directory
$outputDir = ".\output"

# Ensure the output directory exists
if (-Not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir | Out-Null
}

# Build the runner
Write-Host "Building runner..."
docker-compose -f docker\docker-compose.yml up -d --build build-runner

# Check if the runner was built successfully
if (-Not (Test-Path "$outputDir\runner_main")) {
    Write-Host "Failed to build the runner." -ForegroundColor Red
    exit 1
}
Write-Host "Building Runner completed successfully." -ForegroundColor Green
docker-compose -f docker\docker-compose.yml down
# Build the library
Write-Host "Building library..."
docker-compose -f docker\docker-compose.yml up -d --build build-library

# Check if the library was built successfully
if (-Not (Test-Path "$outputDir\libflow_project_01.so")) {
    Write-Host "Failed to build the library." -ForegroundColor Red
    exit 1
}
Write-Host "Building Library completed successfully." -ForegroundColor Green
docker-compose -f docker\docker-compose.yml down
Write-Host "Build completed successfully." -ForegroundColor Green