// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Card, CardHeader, CardContent } from "@mui/material";
import Form, { FormSubmit } from "@rjsf/core";
import { invoke } from "@tauri-apps/api";
import { useState, useEffect, useMemo, useRef } from "react";
import { tauri_host } from "../tauri_host";

export default function MoveForm() {
  const [schema, set_schema] = useState<null | any>(null);

  useEffect(() => {
    (async () => {
      set_schema(await tauri_host.get_move_schema());
    })()

  }, []);
  console.log("move schema:", schema);
  const handle_submit = (data: FormSubmit) => {
    // TODO: Submit from correct user
    tauri_host.make_move_inner(data.formData)
  };

  const schema_form = useMemo<JSX.Element>(() => {
    const customFormats = { "uint128": (s: string) => { return true; } };
    if (schema)
      return <Form schema={schema} noValidate={true} liveValidate={false} onSubmit={handle_submit} customFormats={customFormats}>
        <button type="submit">Submit</button>
      </Form>;

    else
      return <div></div>
  }
    , [schema]
  )
  return schema && <Card>
    <CardHeader
      title={'Enter A Move'}
      subheader={'Enter a move'}
    >
    </CardHeader>
    <CardContent>
      <div className='MoveForm' >
        {schema_form}
      </div>
    </CardContent>
  </Card>;
}