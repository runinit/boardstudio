import React from 'react';
import { setupGuideStages, type SetupGuideStage, type SetupGuideStages } from './setupGuide';

type SetupGuideProps = {
  boardName: string;
  stages: SetupGuideStages;
  currentStep: SetupGuideStage;
  onStepChange: (step: SetupGuideStage) => void;
  onDismiss: () => void;
  currentStepContent?: React.ReactNode;
};

const labels: Record<SetupGuideStage, string> = { project: 'Project & hardware', layout: 'Layout & assemblies', wiring: 'Controller & wiring', case: 'Case (optional)', review: 'Review & export' };

export function SetupGuide({ boardName, stages, currentStep, onStepChange, onDismiss, currentStepContent }: SetupGuideProps) {
  return <section className="wb-setup-guide" aria-label="Project setup guide">
    <header className="wb-setup-guide__header"><div><h2 tabIndex={-1}>Setup guide</h2><p>{boardName}</p></div><button type="button" className="wb-setup-guide__skip" onClick={onDismiss}>Skip guide</button></header>
    <p className="wb-setup-guide__intro">Set up this board, or return to Objects at any time.</p>
    <nav className="wb-setup-guide__steps" aria-label="Setup steps">{setupGuideStages.map((stage, index) => <button type="button" key={stage} className={`wb-setup-guide__step${stage === currentStep ? ' is-current' : ''}${stages[stage].ready ? ' is-ready' : ''}`} aria-current={stage === currentStep ? 'step' : undefined} onClick={() => onStepChange(stage)}><span className="wb-setup-guide__step-marker">{index + 1}</span><span className="wb-setup-guide__step-copy"><strong>{labels[stage]}</strong>{(stage === currentStep || stages[stage].ready) && <small>{stages[stage].detail}</small>}</span></button>)}</nav>
    <div className="wb-setup-guide__content">{currentStepContent}</div>
  </section>;
}
