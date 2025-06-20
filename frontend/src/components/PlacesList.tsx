import { createSignal, onMount, For, Show } from 'solid-js';
import type { Place } from '../types';

const PlacesList = () => {
  const [places, setPlaces] = createSignal<Place[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      const response = await fetch('/api/places'); // Assuming backend is proxied to /api
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data = await response.json();
      setPlaces(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  });

  return (
    <div class="p-4">
      <h2 class="text-2xl font-bold mb-4">Places to Visit</h2>
      <Show when={loading()}>
        <p>Loading places...</p>
      </Show>
      <Show when={error()}>
        <p class="text-red-500">Error fetching places: {error()}</p>
      </Show>
      <Show when={!loading() && !error() && places().length === 0}>
        <p>No places found.</p>
      </Show>
      <ul class="space-y-2">
        <For each={places()}>
          {(place) => (
            <li class="p-2 border rounded shadow-sm">
              <h3 class="text-lg font-semibold">{place.name}</h3>
              <p class="text-gray-700">{place.description}</p>
              <Show when={place.location}>
                 <p class="text-sm text-gray-500">Location: {place.location}</p>
              </Show>
            </li>
          )}
        </For>
      </ul>
    </div>
  );
};

export default PlacesList;
