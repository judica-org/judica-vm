import React from "react";
import countries_data from "./countries.json";
import earth from "./earth-dark.jpeg";
import Globe from "react-globe.gl";
const { useState, useEffect } = React;
export default () => {
    const [countries, setCountries] = useState({ features: [] });

    useEffect(() => {
        // load data
        setCountries(countries_data)
    }, []);

    return <Globe
    globeImageUrl = { earth }

    hexPolygonsData = { countries.features }
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
};