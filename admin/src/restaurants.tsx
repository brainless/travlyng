import * as React from "react";
import {
    List,
    Datagrid,
    TextField,
    EditButton,
    SimpleForm,
    TextInput,
    Create,
    Edit,
    useRecordContext
} from 'react-admin';
import { RichTextInput } from 'ra-input-rich-text';

export const RestaurantList = () => (
    <List>
        <Datagrid>
            <TextField source="id" />
            <TextField source="name" />
            <TextField source="location" />
            <TextField source="description" /> {/* Using TextField for now, can be RichTextField if needed */}
            <EditButton />
        </Datagrid>
    </List>
);

export const RestaurantEdit = () => (
    <Edit title={<RestaurantTitle />}>
        <SimpleForm>
            <TextInput source="id" disabled />
            <TextInput source="name" fullWidth />
            <TextInput source="location" fullWidth />
            <RichTextInput source="description" fullWidth />
        </SimpleForm>
    </Edit>
);

export const RestaurantCreate = () => (
    <Create>
        <SimpleForm>
            <TextInput source="name" fullWidth />
            <TextInput source="location" fullWidth />
            <RichTextInput source="description" fullWidth />
        </SimpleForm>
    </Create>
);

const RestaurantTitle = () => {
    const record = useRecordContext();
    return <span>Restaurant {record ? `"${record.name}"` : ''}</span>;
};
