// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import autoprefixer from "autoprefixer";
import postcss from "postcss";
import React, { MouseEvent, ReactNode, useCallback } from "react";
import { Control, useFormContext } from "react-hook-form";
import sass, { Logger } from "sass";

import { IStyleSheet, StyleSheet } from "../../apis/strims/chat/v1/chat";
import { fromFormImageValue, toFormImageValue } from "../../lib/image";
import {
  Button,
  ButtonSet,
  ImageInput,
  ImageValue,
  InputLabel,
  TextAreaInput,
  TextInput,
} from "../Form";

export interface ChatStyleSheetFormData {
  scss: string;
  assetCount: number;
  asset_0_name?: string;
  asset_0_image?: ImageValue;
  asset_1_name?: string;
  asset_1_image?: ImageValue;
  asset_2_name?: string;
  asset_2_image?: ImageValue;
  asset_3_name?: string;
  asset_3_image?: ImageValue;
}

export const ChatStyleSheetInput: React.FC = () => {
  const { control, setValue, getValues, watch } = useFormContext<ChatStyleSheetFormData>();

  const handleAddAssetClick = useCallback((e: MouseEvent) => {
    e.preventDefault();
    setValue("assetCount", (getValues().assetCount >> 0) + 1);
  }, []);

  const handleAssetDelete = useCallback((i: AssetIndex) => {
    const values = getValues();
    for (let j = i; j < 4; j++) {
      const k = (j + 1) as AssetIndex;
      setValue(`asset_${j}_name`, values[`asset_${k}_name`]);
      setValue(`asset_${j}_image`, values[`asset_${k}_image`]);
    }
    setValue("assetCount", values.assetCount - 1);
  }, []);

  const assetCount = watch("assetCount");

  const assetInputs: ReactNode[] = [];
  for (let i = 0; i < assetCount; i++) {
    assetInputs.push(
      <AssetInput control={control} key={i} index={i as AssetIndex} onDelete={handleAssetDelete} />
    );
  }

  return (
    <>
      <TextAreaInput
        control={control}
        rules={{
          validate: (v: string) => {
            try {
              compileSCSS(v);
            } catch (e) {
              return false;
            }
          },
        }}
        name="scss"
        label="SCSS"
      />
      {...assetInputs}
      <ButtonSet>
        <Button disabled={assetCount >= 4} onClick={handleAddAssetClick}>
          Add Asset
        </Button>
      </ButtonSet>
    </>
  );
};

type AssetIndex = 0 | 1 | 2 | 3;

interface AssetInputProps {
  control: Control<ChatStyleSheetFormData>;
  index: AssetIndex;
  onDelete: (i: number) => void;
}

const AssetInput: React.FC<AssetInputProps> = ({ control, index, onDelete }) => {
  const handleDeleteClick = useCallback((e: MouseEvent) => {
    e.preventDefault();
    onDelete(index);
  }, []);

  return (
    <>
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Asset name required",
          },
        }}
        name={`asset_${index}_name`}
        label="Name"
        placeholder="Enter an asset name"
      />
      <InputLabel text="image" component="div">
        <ImageInput
          control={control}
          name={`asset_${index}_image`}
          maxSize={10485764}
          rules={{
            required: {
              value: true,
              message: "Image is required",
            },
          }}
        />
      </InputLabel>
      <ButtonSet>
        <Button onClick={handleDeleteClick}>Remove Asset</Button>
      </ButtonSet>
    </>
  );
};

export const fromStyleSheetFormValue = async (
  data: ChatStyleSheetFormData
): Promise<IStyleSheet> => {
  const { css } = await postcss([autoprefixer]).process(compileSCSS(data.scss));

  const assets: StyleSheet.IAsset[] = [];
  for (let i = 0 as AssetIndex; i < data.assetCount; i++) {
    assets.push({
      name: data[`asset_${i}_name`],
      image: fromFormImageValue(data[`asset_${i}_image`]),
    });
  }

  return {
    scss: data.scss,
    css,
    assets,
  };
};

export const toStyleSheetFormValue = (styleSheet?: StyleSheet): ChatStyleSheetFormData => {
  const data: ChatStyleSheetFormData = {
    scss: styleSheet?.scss,
    assetCount: styleSheet?.assets?.length >> 0,
  };
  for (let i = 0 as AssetIndex; i < styleSheet?.assets?.length; i++) {
    data[`asset_${i}_name`] = styleSheet.assets[i].name;
    data[`asset_${i}_image`] = toFormImageValue(styleSheet.assets[i].image);
  }
  return data;
};

export const compileSCSS = (scss: string) => {
  const res = sass.compileString(scss, {
    logger: Logger.silent,
    alertColor: false,
    sourceMap: false,
    quietDeps: true,
    functions: null,
    sourceMapIncludeSources: false,
    importers: null,
  });
  return res.css;
};
