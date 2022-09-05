import { Card, CardHeader, CardContent } from "@mui/material";
import Form, { FormSubmit } from "@rjsf/core";
import { invoke } from "@tauri-apps/api";
import { useEffect, useMemo, useRef, useState } from "react";
import { RawMaterialsActions } from "../util";

const PurchaseMaterialForm = ({ action, subtitle, currency }: { readonly action: RawMaterialsActions; readonly subtitle: string; readonly currency: string }) => {
  const [schema, setSchema] = useState<null | any>(null);

  useEffect(() => {
    (async () => {
      setSchema(await invoke("get_materials_schema"));
    })()
  });
  console.log("materials schema:", schema);

  const handle_submit = (data: FormSubmit) => {
    invoke("make_move_inner", {
      nextMove: data.formData,
      from: uid.current?.valueAsNumber 
    });
  };

  const formData = {
    currency
  }

  // for creater should be extracted out into a form util
  const schema_form = useMemo<JSX.Element>(() => {
    const customFormats = { "uint128": (s: string) => { return true; } };
    if (schema)
      return <Form formData={formData} schema={schema} noValidate={true} liveValidate={false} onSubmit={handle_submit} customFormats={customFormats}>
        <button type="submit">Submit</button>
      </Form>;

    else
      return <div></div>
  }
    , [schema]
  )
  const uid = useRef<null | HTMLInputElement>(null);
  return schema && <Card>
    <CardHeader
      title={action}
      subheader={subtitle}
    >
    </CardHeader>
    <CardContent>
      <div className='MoveForm' >
        <div>
          <label>Player ID:</label>
          <input type={"number"} ref={uid}></input>
        </div>
        {schema_form}
      </div>
    </CardContent>
  </Card>;
};

export default PurchaseMaterialForm;

