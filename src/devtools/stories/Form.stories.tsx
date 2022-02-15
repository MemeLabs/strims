import React from "react";
import { useForm } from "react-hook-form";

import {
  Button,
  ButtonSet,
  CreatableSelectInput,
  ImageInput,
  ImageValue,
  InputLabel,
  SelectInput,
  SelectOption,
  TextAreaInput,
  TextInput,
  ToggleInput,
} from "../../components/Form";

const selectOptions: SelectOption<string>[] = [
  {
    label: "test 0",
    value: "0",
  },
  {
    label: "test 1",
    value: "1",
  },
  {
    label: "test 2",
    value: "2",
  },
  {
    label: "test 3",
    value: "3",
  },
];

interface InputStoryFormData {
  text: string;
  number: number;
  boolean: boolean;
  textarea: string;
  select: SelectOption<string>;
  creatableSelect: SelectOption<string>[];
  image: ImageValue;
}

const InputStory: React.FC = () => {
  const { handleSubmit, control } = useForm<InputStoryFormData>({
    mode: "onBlur",
    defaultValues: {
      text: "test value",
      number: 123,
      boolean: true,
      textarea: "test textarea value",
      select: selectOptions[0],
      creatableSelect: selectOptions,
    },
  });

  const onSubmit = (data: InputStoryFormData) => console.log(data);

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      <TextInput control={control} label="text" name="text" />
      <TextInput control={control} label="number" name="number" type="number" />
      <ToggleInput control={control} label="boolean" name="boolean" />
      <TextAreaInput control={control} label="textarea" name="textarea" />
      <SelectInput control={control} options={selectOptions} label="select" name="select" />
      <CreatableSelectInput control={control} label="creatable select" name="creatableSelect" />
      <InputLabel text="image" component="div">
        <ImageInput control={control} name="image" />
      </InputLabel>
      <ButtonSet>
        <Button>Save</Button>
      </ButtonSet>
    </form>
  );
};

export default [
  {
    name: "Inputs",
    component: () => <InputStory />,
  },
];
