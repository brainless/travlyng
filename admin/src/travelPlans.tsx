import * as React from "react";
import {
    List,
    Datagrid,
    TextField,
    DateField,
    EditButton,
    DeleteButton,
    SimpleForm,
    TextInput,
    DateInput,
    Create,
    Edit,
    ReferenceManyField,
    useRecordContext,
    ReferenceInput,
    SelectInput,
    NumberInput,
    TopToolbar,
    CreateButton,
    ExportButton,
    Button,
    useRedirect,
    useNotify,
    required,
    RaRecord
} from 'react-admin';
import { RichTextInput } from 'ra-input-rich-text';
import { Link } from 'react-router-dom';


// --- PlanItem Components (for use within TravelPlanEdit) ---

const PlanItemCreateButton = () => {
    const record = useRecordContext(); // Gets the current TravelPlan record
    if (!record) return null;
    return (
        <Button
            component={Link}
            to={{
                pathname: '/plan_items/create',
                state: { record: { plan_id: record.id } }, // Pass plan_id for pre-filling the form
            }}
            label="Add Plan Item"
        />
    );
};

const PlanItemEditButton = () => {
    const record = useRecordContext(); // Gets the current PlanItem record
    if (!record || !record.plan_id) { // Ensure plan_id is in the record (should be from getManyReference)
         // If plan_id is missing, editing might fail due to dataProvider needing it.
         // This assumes your getManyReference for plan_items returns plan_id in each item.
        console.warn("PlanItem record is missing plan_id, Edit may not work correctly.", record);
    }
    return (
        <EditButton
            record={record} // Pass the current record to the edit button
        />
    );
};


// --- TravelPlan Components ---

export const TravelPlanList = () => (
    <List>
        <Datagrid rowClick="edit">
            <TextField source="id" />
            <TextField source="name" />
            <DateField source="start_date" />
            <DateField source="end_date" />
            <EditButton />
        </Datagrid>
    </List>
);

const TravelPlanEditActions = () => (
    <TopToolbar>
        <PlanItemCreateButton />
        <ExportButton />
    </TopToolbar>
);

export const TravelPlanEdit = () => (
    <Edit title={<TravelPlanTitle />} actions={<TravelPlanEditActions />}>
        <SimpleForm>
            <TextInput source="id" disabled />
            <TextInput source="name" fullWidth validate={required()} />
            <DateInput source="start_date" fullWidth />
            <DateInput source="end_date" fullWidth />

            <ReferenceManyField
                label="Itinerary Items"
                reference="plan_items" // Resource name for plan items
                target="plan_id"       // Foreign key in plan_items that points to the current plan's id
                sort={{ field: 'visit_date', order: 'ASC' }}
            >
                <Datagrid>
                    <TextField source="id" label="Item ID" />
                    <TextField source="entity_type" />
                    <TextField source="entity_id" /> {/* TODO: Could be a ReferenceField to actual entity if desired */}
                    <DateField source="visit_date" />
                    <TextField source="notes" />
                    <PlanItemEditButton />
                    <DeleteButton mutationMode="pessimistic" />
                </Datagrid>
            </ReferenceManyField>
        </SimpleForm>
    </Edit>
);

export const TravelPlanCreate = () => (
    <Create>
        <SimpleForm>
            <TextInput source="name" fullWidth validate={required()} />
            <DateInput source="start_date" fullWidth />
            <DateInput source="end_date" fullWidth />
            {/* Items are added after creation via the Edit view */}
        </SimpleForm>
    </Create>
);

const TravelPlanTitle = () => {
    const record = useRecordContext();
    return <span>Travel Plan {record ? `"${record.name}"` : ''}</span>;
};


// --- PlanItem Resource Components (standalone, but aware of plan_id context) ---

export const PlanItemList = (props: any) => (
    // This list shows ALL plan items.
    // Filtering by plan_id here would require backend support for /plan_items?filter={"plan_id":X}
    // or a custom getList in dataProvider that fetches from /plans/:id/items if a plan_id filter is active.
    // For now, it's a global list.
    <List {...props} title="All Plan Itinerary Items">
        <Datagrid rowClick="edit">
            <TextField source="id" />
            <ReferenceInput source="plan_id" reference="plans">
                <TextField source="name" />
            </ReferenceInput>
            <TextField source="entity_type" />
            <TextField source="entity_id" />
            <DateField source="visit_date" />
            <TextField source="notes" />
            <EditButton />
            <DeleteButton mutationMode="pessimistic" />
        </Datagrid>
    </List>
);

export const PlanItemEdit = (props: any) => {
    const record = useRecordContext();
    const notify = useNotify();
    const redirect = useRedirect();

    // The customDataProvider for 'update' needs 'plan_id' in params.data.
    // If record.plan_id is not already in the form data for submission,
    // we might need to ensure it's included, or rely on it being part of the record.
    // The getOne for PlanItem in customDataProvider is problematic.
    // This Edit view likely works best when navigated from TravelPlanEdit (where plan_id is in the record).
    // If navigating from a general list, the record fetched by getOne might lack plan_id.
    // The backend /plan_items/:id should ideally return plan_id.

    const transform = (data: any) => {
        // Ensure plan_id is part of the data payload for the custom dataProvider update
        if (record && record.plan_id && !data.plan_id) {
            return { ...data, plan_id: record.plan_id };
        }
        return data;
    };

    return (
        <Edit {...props} title={<PlanItemTitle />} transform={transform} mutationMode="pessimistic"
            onSuccess={() => {
                notify('Plan item updated');
                redirect('show', 'plans', record?.plan_id); // Redirect to the parent plan's show view or edit
            }}
        >
            <SimpleForm>
                <TextInput source="id" disabled />
                <ReferenceInput source="plan_id" reference="plans" disabled>
                    <SelectInput optionText="name" />
                </ReferenceInput>
                <TextInput source="entity_type" validate={required()} /> {/* Could be a SelectInput with known types: 'place', 'accommodation', 'restaurant' */}
                <NumberInput source="entity_id" validate={required()} /> {/* Could be a ReferenceInput based on entity_type selection */}
                <DateInput source="visit_date" />
                <RichTextInput source="notes" fullWidth />
            </SimpleForm>
        </Edit>
    );
};

export const PlanItemCreate = (props: any) => {
    const notify = useNotify();
    const redirect = useRedirect();
    const locationState = props.location?.state as { record?: { plan_id?: number } } | undefined;
    const initialPlanId = locationState?.record?.plan_id;

    // The customDataProvider for 'create' needs 'plan_id' in params.data.
    // We pre-fill it if passed in state (e.g. from TravelPlanEdit's "Add Plan Item" button)
    const transform = (data: any) => ({
        ...data,
        plan_id: data.plan_id || initialPlanId // Ensure plan_id is in the submitted data
    });

    return (
        <Create
            {...props}
            title="Create Plan Item"
            transform={transform}
            record={{ plan_id: initialPlanId }} // Pass initialValues as record to Create
            onSuccess={(record: RaRecord) => {
                notify('Plan item created');
                redirect('show', 'plans', record?.plan_id); // Redirect to the parent plan's show view or edit
            }}
        >
            <SimpleForm>
                {/* If initialPlanId is undefined, user must select/enter a plan_id */}
                {initialPlanId ? (
                    // Note: ReferenceInput's initialValue might be redundant if Create's record pre-fills it.
                    // However, explicit initialValue on ReferenceInput can be useful for display before the form fully initializes.
                    <ReferenceInput source="plan_id" reference="plans" initialValue={initialPlanId} disabled>
                         <SelectInput optionText="name" />
                    </ReferenceInput>
                ) : (
                    <ReferenceInput source="plan_id" reference="plans" validate={required() as any}>
                        <SelectInput optionText="name" />
                    </ReferenceInput>
                )}
                <TextInput source="entity_type" validate={required()} />
                <NumberInput source="entity_id" validate={required()} />
                <DateInput source="visit_date" />
                <RichTextInput source="notes" fullWidth />
            </SimpleForm>
        </Create>
    );
};

const PlanItemTitle = () => {
    const record = useRecordContext();
    return <span>Plan Item {record ? `ID: ${record.id}` : ''} (Plan: {record?.plan_id})</span>;
};
