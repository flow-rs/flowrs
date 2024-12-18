# Step 1: Build the runner and library
Write-Host "Step 1: Building the runner and library..."
.\build.ps1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed. Exiting workflow." -ForegroundColor Red
    exit 1
}

# Step 2: Run the distributed tests
Write-Host "Step 2: Running distributed tests..."
.\run_tests.ps1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Distributed tests failed." -ForegroundColor Red
    exit 1
}

Write-Host "Workflow completed successfully." -ForegroundColor Green