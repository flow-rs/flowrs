# Variables
$outputDir = "D:\Moritz\Documents\Studium\2024\flowrs\tests\output"
$composeFile = "docker\docker-compose.yml"

# Ensure the output directory exists
if (-Not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir | Out-Null
}

# Function to build a service, verify files inside, and extract them
function Build-Service {
    param (
        [string]$serviceName,
        [string]$expectedFile,
        [string]$containerPath
    )

    Write-Host "Building $serviceName..."
        # Force Docker to use BuildKit and respect .dockerignore
    $env:DOCKER_BUILDKIT = "1"
    docker-compose -f $composeFile up -d --build $serviceName

    # Find the latest image ID for this service
    $imageId = docker images --format "{{.ID}} {{.Repository}}:{{.Tag}}" | Select-String -Pattern "docker-${serviceName}" | ForEach-Object { $_ -split " " } | Select-Object -First 1

    if (-not $imageId) {
        Write-Host "Error: Could not find image for ${serviceName}" -ForegroundColor Red
        exit 1
    }

    # Create a temporary container that stays alive
    Write-Host "Creating temporary container from image $imageId..."
    $containerId = docker run -d --entrypoint /bin/sh $imageId -c "while true; do sleep 10; done"

    if (-not $containerId -or $containerId -match "Error") {
        Write-Host "Error: Could not start container from image $imageId" -ForegroundColor Red
        exit 1
    }

    # List files inside /output
    Write-Host "Listing files in /output of container $containerId..."
    docker exec $containerId ls -lah /output

    # Check if the file exists before copying
    $fileExists = docker exec $containerId sh -c "[ -f $containerPath ] && echo 'exists' || echo 'not found'"

    if ($fileExists -match "not found") {
        Write-Host "Error: $containerPath does NOT exist inside the container!" -ForegroundColor Red
        docker stop $containerId | Out-Null
        docker rm $containerId | Out-Null
        exit 1
    }

    # Ensure the container is still running before copying
    if (-not (docker ps -q --filter "id=$containerId")) {
        Write-Host "Error: Container stopped before copying files!" -ForegroundColor Red
        exit 1
    }

    # Extract file from the temporary container
    Write-Host "Extracting $expectedFile from container $containerId..."
    docker cp "${containerId}:$containerPath" "$expectedFile"

    if (-Not (Test-Path $expectedFile)) {
        Write-Host "Failed to copy $expectedFile from container $containerId" -ForegroundColor Red
        docker stop $containerId | Out-Null
        docker rm $containerId | Out-Null
        exit 1
    }

    Write-Host "Building $serviceName completed successfully and file extracted." -ForegroundColor Green

    # Clean up
    docker stop $containerId | Out-Null
    docker rm $containerId | Out-Null
}

# Build the runner main
Build-Service "build-runner" "$outputDir\runner_main" "/output/runner_main"

# Build and extract the library
Build-Service "build-library" "$outputDir\libflow_project.so" "/output/libflow_project.so"

# Build the base-runtime (no file extraction needed)
Write-Host "Building base-runtime..."
docker-compose -f $composeFile up -d --build base-runtime

if ($LASTEXITCODE -ne 0) {
    Write-Host "Failed to build base-runtime. Docker Compose returned an error." -ForegroundColor Red
    exit 1
}

Write-Host "Building base-runtime completed successfully." -ForegroundColor Green

# Shutdown all containers in a safe way
docker-compose -f $composeFile stop build-library
docker-compose -f $composeFile rm -f build-library

Write-Host "All builds completed successfully." -ForegroundColor Green
