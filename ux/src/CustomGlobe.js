import React from "react";
import { useEffect, useState, useRef } from "react";
import HEX_DATA from "./countries.json";
import Globe from "react-globe.gl";

// export const MakeGlobe = React.memo(function CustomGlobe() {
//   // const globeEl = useRef();
//   const [hex, setHex] = useState({ features: [] });

//   useEffect(() => {
//     setHex(HEX_DATA);
//   }, []);

//   // useEffect(() => {
//   //   const MAP_CENTER = { lat: 0, lng: 0, altitude: 1.5 };
//   //   globeEl.current.pointOfView(MAP_CENTER, 0);
//   // }, [globeEl]);

//   return (
//     <Globe
//       width={500}
//       height={500}
//       // ref={globeEl}
//       // backgroundColor="rgba(0,0,0,0)"
//       globeImageUrl={"//unpkg.com/three-globe/example/img/earth-dark.jpg"}
//       hexPolygonsData={hex.features}
//       hexPolygonResolution={3}
//       hexPolygonMargin={0.62}
//        hexPolygonColor={() => `#${Math.round(Math.random() * Math.pow(2, 24)).toString(16).padStart(6, '0')}`}
//       hexPolygonLabel={({ properties: d }) => `
//         <b>${d.ADMIN} (${d.ISO_A2})</b> <br />
//         Population: <i>${d.POP_EST}</i>
//       `}
//     />
//   );
// })

export default function CustomGlobe() {
  // const globeEl = useRef();
  const [hex, setHex] = useState({ features: [] });

  useEffect(() => {
    setHex(HEX_DATA);
  }, []);

  // useEffect(() => {
  //   const MAP_CENTER = { lat: 0, lng: 0, altitude: 1.5 };
  //   globeEl.current.pointOfView(MAP_CENTER, 0);
  // }, [globeEl]);

  return <Globe
      width={500}
      height={500}
      // ref={globeEl}
      // backgroundColor="rgba(0,0,0,0)"
      // globeImageUrl={"//unpkg.com/three-globe/example/img/earth-dark.jpg"}
      hexPolygonResolution = { 3 }
      hexPolygonMargin = { 0.3 }
      hexPolygonColor = {
          () => `#${Math.round(Math.random() * Math.pow(2, 24)).toString(16).padStart(6, '0')}`
      }
      hexPolygonLabel = {
          ({ properties: d }) => `
          <b>${d.ADMIN} (${d.ISO_A2})</b> <br />
          Population: <i>${d.POP_EST}</i>
        `
      }
      />;
}
