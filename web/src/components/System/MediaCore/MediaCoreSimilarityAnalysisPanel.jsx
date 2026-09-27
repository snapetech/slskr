import React from 'react';
import MediaCoreHashSimilarityPanel from './MediaCoreHashSimilarityPanel';
import MediaCoreFuzzyContentPanel from './MediaCoreFuzzyContentPanel';
import MediaCorePerceptualSimilarityPanel from './MediaCorePerceptualSimilarityPanel';
import MediaCoreTextSimilarityPanel from './MediaCoreTextSimilarityPanel';

const MediaCoreSimilarityAnalysisPanel = React.memo(() => (
  <>
    <MediaCoreHashSimilarityPanel />
    <MediaCoreFuzzyContentPanel />
    <MediaCorePerceptualSimilarityPanel />
    <MediaCoreTextSimilarityPanel />
  </>
));

export default MediaCoreSimilarityAnalysisPanel;
