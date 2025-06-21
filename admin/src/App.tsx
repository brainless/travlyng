import * as React from "react";
import { Admin, Resource, DataProvider, fetchUtils } from 'react-admin';
import simpleRestProvider from 'ra-data-simple-rest';

// Icons
import PlaceIcon from '@mui/icons-material/Place';
import HotelIcon from '@mui/icons-material/Hotel';
import RestaurantIcon from '@mui/icons-material/Restaurant';
import EventNoteIcon from '@mui/icons-material/EventNote';
import ListItemIcon from '@mui/icons-material/ListAlt';


import { PlaceList, PlaceEdit, PlaceCreate } from './places';
import { AccommodationList, AccommodationEdit, AccommodationCreate } from './accommodations';
import { RestaurantList, RestaurantEdit, RestaurantCreate } from './restaurants';
import { TravelPlanList, TravelPlanEdit, TravelPlanCreate, PlanItemList, PlanItemEdit, PlanItemCreate } from './travelPlans';

import './App.css';

const API_URL = 'http://localhost:8080';
const baseDataProvider = simpleRestProvider(API_URL);

const httpClient = fetchUtils.fetchJson;

const customDataProvider: DataProvider = {
    ...baseDataProvider,
    getList: (resource, params) => {
        if (resource === 'plan_items' && params.filter && params.filter.plan_id) {
            const { plan_id, ...otherFilters } = params.filter;
            console.log(`getList for ${resource} with plan_id: ${plan_id}. Current simpleRestProvider will call /plan_items. This might not be what's intended for a filtered list.`);
            return baseDataProvider.getList(resource, { ...params, filter: otherFilters });
        }
        return baseDataProvider.getList(resource, params);
    },
    getOne: (resource, params) => {
        if (resource === 'plan_items') {
            console.warn(`getOne for plan_items with ID ${params.id}. This will call ${API_URL}/plan_items/${params.id}. The API expects /plans/:plan_id/items/:item_id.`);
        }
        return baseDataProvider.getOne(resource, params);
    },
    create: async (resource, params) => {
        if (resource === 'plan_items' && params.data && params.data.plan_id) {
            const { plan_id, ...newItemData } = params.data;
            const url = `${API_URL}/plans/${plan_id}/items`;
            const { json } = await httpClient(url, {
                method: 'POST',
                body: JSON.stringify(newItemData),
            });
            return { data: { ...json, id: json.id } };
        }
        return baseDataProvider.create(resource, params);
    },
    update: async (resource, params) => {
        if (resource === 'plan_items' && params.data && params.data.plan_id && params.id) {
            const { plan_id, ...itemDataToUpdate } = params.data;
            const url = `${API_URL}/plans/${plan_id}/items/${params.id}`;
            const { json } = await httpClient(url, {
                method: 'PUT',
                body: JSON.stringify(itemDataToUpdate),
            });
            return { data: { ...json, id: json.id } };
        }
        return baseDataProvider.update(resource, params);
    },
    delete: async (resource, params) => {
        if (resource === 'plan_items' && params.previousData && params.previousData.plan_id && params.id) {
            const { plan_id } = params.previousData;
            const url = `${API_URL}/plans/${plan_id}/items/${params.id}`;
            await httpClient(url, {
                method: 'DELETE',
            });
            return { data: params.previousData };
        }
        return baseDataProvider.delete(resource, params);
    },
    getManyReference: (resource, params) => {
        if (resource === 'plan_items' && params.target === 'plan_id' && params.id) {
            const planId = params.id;
            const url = `${API_URL}/plans/${planId}/items`;
            const query: any = { // Changed to any to allow arbitrary string keys
                _sort: params.sort.field,
                _order: params.sort.order,
                _start: (params.pagination.page - 1) * params.pagination.perPage,
                _end: params.pagination.page * params.pagination.perPage,
            };
            Object.keys(params.filter).forEach(key => {
                query[key] = params.filter[key];
            });
            const finalUrl = `${url}?${fetchUtils.queryParameters(query)}`;

            return httpClient(finalUrl).then(({ headers, json }) => {
                if (!headers.has('x-total-count')) {
                    console.warn(
                        'The X-Total-Count header is missing in the HTTP Response. The jsonServer Data Provider expects it for pagination. If you are using CORS, did you declare X-Total-Count in the Access-Control-Expose-Headers header?'
                    );
                }
                return {
                    data: json.map((item: any) => ({ ...item, id: item.id })),
                    total: parseInt(
                        (headers.get('x-total-count') || "0").split('/').pop() || "0",
                        10
                    ),
                };
            });
        }
        return baseDataProvider.getManyReference(resource, params);
    }
};


const App = () => (
  <Admin dataProvider={customDataProvider}>
    <Resource name="places" list={PlaceList} edit={PlaceEdit} create={PlaceCreate} options={{ label: 'Places to Visit' }} icon={PlaceIcon} />
    <Resource name="accommodations" list={AccommodationList} edit={AccommodationEdit} create={AccommodationCreate} options={{ label: 'Accommodations' }} icon={HotelIcon} />
    <Resource name="restaurants" list={RestaurantList} edit={RestaurantEdit} create={RestaurantCreate} options={{ label: 'Restaurants' }} icon={RestaurantIcon} />
    <Resource name="plans" list={TravelPlanList} edit={TravelPlanEdit} create={TravelPlanCreate} options={{ label: 'Travel Plans' }} icon={EventNoteIcon} />
    <Resource name="plan_items" list={PlanItemList} edit={PlanItemEdit} create={PlanItemCreate} options={{ label: 'Plan Itinerary Items' }} icon={ListItemIcon} />
  </Admin>
);

export default App;
