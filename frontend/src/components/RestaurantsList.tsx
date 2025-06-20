import { createSignal, onMount, For, Show } from 'solid-js';
import type { Restaurant } from '../types';

const RestaurantsList = () => {
  const [restaurants, setRestaurants] = createSignal<Restaurant[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      const response = await fetch('/api/restaurants'); // Assuming backend is proxied to /api
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data = await response.json();
      setRestaurants(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  });

  return (
    <div class="p-4">
      <h2 class="text-2xl font-bold mb-4">Restaurants</h2>
      <Show when={loading()}>
        <p>Loading restaurants...</p>
      </Show>
      <Show when={error()}>
        <p class="text-red-500">Error fetching restaurants: {error()}</p>
      </Show>
      <Show when={!loading() && !error() && restaurants().length === 0}>
        <p>No restaurants found.</p>
      </Show>
      <ul class="space-y-2">
        <For each={restaurants()}>
          {(restaurant) => (
            <li class="p-2 border rounded shadow-sm">
              <h3 class="text-lg font-semibold">{restaurant.name}</h3>
              <p class="text-gray-700">{restaurant.description}</p>
              <Show when={restaurant.location}>
                 <p class="text-sm text-gray-500">Location: {restaurant.location}</p>
              </Show>
            </li>
          )}
        </For>
      </ul>
    </div>
  );
};

export default RestaurantsList;
