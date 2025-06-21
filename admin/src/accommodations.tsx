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

export const AccommodationList = () => (
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

export const AccommodationEdit = () => (
    <Edit title={<AccommodationTitle />}>
        <SimpleForm>
            <TextInput source="id" disabled />
            <TextInput source="name" fullWidth />
            <TextInput source="location" fullWidth />
            <RichTextInput source="description" fullWidth />
        </SimpleForm>
    </Edit>
);

export const AccommodationCreate = () => (
    <Create>
        <SimpleForm>
            <TextInput source="name" fullWidth />
            <TextInput source="location" fullWidth />
            <RichTextInput source="description" fullWidth />
        </SimpleForm>
    </Create>
);

const AccommodationTitle = () => {
    const record = useRecordContext();
    return <span>Accommodation {record ? `"${record.name}"` : ''}</span>;
};
