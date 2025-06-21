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
    ReferenceInput,
    SelectInput,
    NumberInput,
    DateInput,
    RichTextField,
    useRecordContext
} from 'react-admin';
import { RichTextInput } from 'ra-input-rich-text';

export const PlaceList = () => (
    <List>
        <Datagrid>
            <TextField source="id" />
            <TextField source="name" />
            <TextField source="location" />
            <RichTextField source="description" />
            <EditButton />
        </Datagrid>
    </List>
);

export const PlaceEdit = () => (
    <Edit title={<PlaceTitle />}>
        <SimpleForm>
            <TextInput source="id" disabled />
            <TextInput source="name" fullWidth />
            <TextInput source="location" fullWidth/>
            <RichTextInput source="description" fullWidth />
        </SimpleForm>
    </Edit>
);

export const PlaceCreate = () => (
    <Create>
        <SimpleForm>
            <TextInput source="name" fullWidth />
            <TextInput source="location" fullWidth />
            <RichTextInput source="description" fullWidth />
        </SimpleForm>
    </Create>
);

const PlaceTitle = () => {
    const record = useRecordContext();
    return <span>Place {record ? `"${record.name}"` : ''}</span>;
};
