# Deletes the files WestPlay Lite no longer has. Run from the repo root (C:\Users\jhuzt\WestPlayLite).
# Files already gone are skipped.
$ErrorActionPreference = 'SilentlyContinue'
Remove-Item -LiteralPath 'ui\dev-map.html' -Force
Remove-Item -LiteralPath 'ui\src\cabin\apps\calendar\CalendarApp.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\apps\calendar\calendar.css' -Force
Remove-Item -LiteralPath 'ui\src\cabin\apps\weather\WeatherApp.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\apps\weather\weather.css' -Force
Remove-Item -LiteralPath 'ui\src\cabin\dashboard\MapView.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\dashboard\WeatherGlyph.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\dashboard\destination.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\dashboard\destination.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\dashboard\idle.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\dashboard\map.css' -Force
Remove-Item -LiteralPath 'ui\src\cabin\dashboard\mapGrows.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\dashboard\mapGrows.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\dashboard\weather.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\dashboard\weather.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\dashboard\widgetState.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\Destinations.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\GuidanceCard.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\ManeuverGlyph.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\MapControls.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\MapSearch.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\PlaceBadge.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\RoadMap.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\alongRoute.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\alongRoute.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\buildingMeshes.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\buildingMeshes.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\buildings.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\buildings.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\camera.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\camera.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\devView.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\frameCap.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\frameCap.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\labels.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\labels.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\liveMap.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\mapView.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\mapView.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\maps.css' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\nextStop.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\nextStop.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\placeIcons.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\placeIcons.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\raisedRoads.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\recents.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\routeChange.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\style.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\style.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\traffic.test.ts' -Force
Remove-Item -LiteralPath 'ui\src\cabin\maps\traffic.ts' -Force
Remove-Item -LiteralPath 'ui\src\companion\pages\MapPage.tsx' -Force
Remove-Item -LiteralPath 'ui\src\cabin\apps\calendar' -Recurse -Force
Remove-Item -LiteralPath 'ui\src\cabin\apps\weather' -Recurse -Force
Write-Host 'Done: the files WestPlay Lite no longer has are removed.'
