import { createSignal, onMount, For, Show } from 'solid-js';
import type { Accommodation } from '../types';

const AccommodationsList = () => {
  const [accommodations, setAccommodations] = createSignal<Accommodation[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      const response = await fetch('/api/accommodations'); // Assuming backend is proxied to /api
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data = await response.json();
      setAccommodations(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  });

  return (
    <div class="p-4">
      <h2 class="text-2xl font-bold mb-4">Accommodations</h2>
      <Show when={loading()}>
        <p>Loading accommodations...</p>
      </Show>
      <Show when={error()}>
        <p class="text-red-500">Error fetching accommodations: {error()}</p>
      </Show>
      <Show when={!loading() && !error() && accommodations().length === 0}>
        <p>No accommodations found.</p>
      </Show>
      <ul class="space-y-2">
        <For each={accommodations()}>
          {(accommodation) => (
            <li class="p-2 border rounded shadow-sm">
              <h3 class="text-lg font-semibold">{accommodation.name}</h3>
              <p class="text-gray-700">{accommodation.description}</p>
              <Show when={accommodation.location}>
                 <p class="text-sm text-gray-500">Location: {accommodation.location}</p>
              </Show>
            </li>
          )}
        </For>
      </ul>
    </div>
  );
};

export default AccommodationsList;
