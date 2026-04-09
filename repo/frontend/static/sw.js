// Service Worker for Fund Transparency Portal — offline-first strategy
const CACHE_NAME = 'fund-transparency-v1';
const STATIC_ASSETS = [
    '/',
    '/index.html',
    '/static/style.css',
    '/manifest.json',
];

// Install: pre-cache static shell
self.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open(CACHE_NAME).then((cache) => cache.addAll(STATIC_ASSETS))
    );
    self.skipWaiting();
});

// Activate: clean old caches
self.addEventListener('activate', (event) => {
    event.waitUntil(
        caches.keys().then((keys) =>
            Promise.all(
                keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k))
            )
        )
    );
    self.clients.claim();
});

// Fetch: network-first for API, cache-first for static assets
self.addEventListener('fetch', (event) => {
    const url = new URL(event.request.url);

    // API requests: network-first with cache fallback
    if (url.pathname.startsWith('/api/')) {
        // Only cache GET API requests
        if (event.request.method === 'GET') {
            event.respondWith(
                fetch(event.request)
                    .then((response) => {
                        if (response.ok) {
                            const clone = response.clone();
                            caches.open(CACHE_NAME).then((cache) => cache.put(event.request, clone));
                        }
                        return response;
                    })
                    .catch(() => caches.match(event.request))
            );
        }
        // Non-GET API requests pass through (mutations need connectivity)
        return;
    }

    // Static assets & SPA shell: cache-first with network fallback
    event.respondWith(
        caches.match(event.request).then((cached) => {
            if (cached) return cached;
            return fetch(event.request).then((response) => {
                if (response.ok) {
                    const clone = response.clone();
                    caches.open(CACHE_NAME).then((cache) => cache.put(event.request, clone));
                }
                return response;
            });
        }).catch(() => {
            // For navigation requests, return the cached shell
            if (event.request.mode === 'navigate') {
                return caches.match('/index.html');
            }
        })
    );
});
