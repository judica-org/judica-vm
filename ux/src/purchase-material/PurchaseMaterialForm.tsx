import { Card, CardHeader, CardContent } from "@mui/material";
import Form, { FormSubmit } from "@rjsf/core";
import { invoke } from "@tauri-apps/api";
import { useEffect, useMemo, useRef, useState } from "react";
import { tauri_host } from "../tauri_host";
import { RawMaterialsActions } from "../util";

const PurchaseMaterialForm = ({ action, subtitle, currency }: { readonly action: RawMaterialsActions; readonly subtitle: string; readonly currency: string }) => {
  const [schema, setSchema] = useState<null | any>(null);

  useEffect(() => {
    (async () => {
      setSchema(await tauri_host.get_material_schema());
    })()
  }, []);
  console.log("materials schema:", schema);

  const handle_submit = (data: FormSubmit) => {

    if (uid.current?.valueAsNumber)
      tauri_host.make_move_inner(data.formData, uid.current?.valueAsNumber)
  };


  // for creater should be extracted out into a form util
  const schema_form = useMemo<JSX.Element>(() => {
    const formData = {
      currency
    };
    const customFormats = { "uint128": (s: string) => { return true; } };
    if (schema)
      return <Form formData={formData} schema={schema} noValidate={true} liveValidate={false} onSubmit={handle_submit} customFormats={customFormats}>
        <button type="submit">Submit</button>
      </Form>;

    else
      return <div></div>
  }
    , [currency, schema]
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

