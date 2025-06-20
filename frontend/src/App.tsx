import type { Component } from 'solid-js';
import PlacesList from './components/PlacesList';
import AccommodationsList from './components/AccommodationsList';
import RestaurantsList from './components/RestaurantsList';

const App: Component = () => {
  return (
    <div class="container mx-auto p-4">
      <header class="mb-8">
        <h1 class="text-4xl font-bold text-center text-blue-600">Travel Planner</h1>
      </header>
      <main class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <PlacesList />
        <AccommodationsList />
        <RestaurantsList />
      </main>
    </div>
  );
};

export default App;
