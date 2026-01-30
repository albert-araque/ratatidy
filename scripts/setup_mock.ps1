# setup_mock.ps1
# Creates a mock directory structure with hardlinks for testing ratatidy on Windows

$BaseDir = Join-Path (Get-Location) "mock_env"

# Clean up
if (Test-Path $BaseDir) {
    Remove-Item -Recurse -Force $BaseDir
}

# Create directories
$DownloadsDir = New-Item -ItemType Directory -Path (Join-Path $BaseDir "downloads")
$MoviesDir = New-Item -ItemType Directory -Path (Join-Path $BaseDir "media\movies")
$ShowsDir = New-Item -ItemType Directory -Path (Join-Path $BaseDir "media\tvshows")

# 1. Movie with Hardlink
$MovieSrc = Join-Path $DownloadsDir "Inception.2010.1080p.mkv"
"Dummy content for Inception" | Out-File -FilePath $MovieSrc
$MovieDest = Join-Path $MoviesDir "Inception (2010)\Inception.mkv"
New-Item -ItemType Directory -Path (Split-Path $MovieDest) -Force | Out-Null
New-Item -ItemType HardLink -Path $MovieDest -Value $MovieSrc | Out-Null

# 2. TV Show with multiple episodes
$ShowFolder = New-Item -ItemType Directory -Path (Join-Path $DownloadsDir "The.Bear.S01.1080p")
$Ep1Src = Join-Path $ShowFolder "The.Bear.S01E01.mkv"
$Ep2Src = Join-Path $ShowFolder "The.Bear.S01E02.mkv"
"Content Ep 1" | Out-File -FilePath $Ep1Src
"Content Ep 2" | Out-File -FilePath $Ep2Src

$ShowDestDir = New-Item -ItemType Directory -Path (Join-Path $ShowsDir "The Bear\Season 01") -Force
New-Item -ItemType HardLink -Path (Join-Path $ShowDestDir "The Bear - S01E01.mkv") -Value $Ep1Src | Out-Null
New-Item -ItemType HardLink -Path (Join-Path $ShowDestDir "The Bear - S01E02.mkv") -Value $Ep2Src | Out-Null

# 3. Orphan Download (No hardlink in media)
"Orphan content" | Out-File -FilePath (Join-Path $DownloadsDir "Spam.mkv")

# 4. Orphan Media (No hardlink in downloads)
$OrphanMedia = Join-Path $MoviesDir "Old_Movie.mkv"
"Old movie content" | Out-File -FilePath $OrphanMedia

Write-Host "Mock environment created at: $BaseDir"
Write-Host "You can now point your config to these folders."
