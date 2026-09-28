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
  const stepIndex = setupGuideStages.indexOf(currentStep);
  return <section className="wb-setup-guide" aria-label="Project setup guide">
    <header className="wb-setup-guide__header"><div><h2 tabIndex={-1}>Setup guide</h2><p>{boardName}</p></div><button type="button" className="wb-setup-guide__skip" onClick={onDismiss}>Back to objects</button></header>
    <p className="wb-setup-guide__intro">Step {stepIndex + 1} of {setupGuideStages.length}. Move between steps freely; your work is kept.</p>
    <nav className="wb-setup-guide__steps" aria-label="Setup steps">{setupGuideStages.map((stage, index) => <button type="button" key={stage} className={`wb-setup-guide__step${stage === currentStep ? ' is-current' : ''}${stages[stage].ready ? ' is-ready' : ''}`} aria-current={stage === currentStep ? 'step' : undefined} onClick={() => onStepChange(stage)}><span className="wb-setup-guide__step-marker">{stages[stage].ready ? <svg viewBox="0 0 20 20" aria-label="Ready"><path d="m4 10 4 4 8-8" /></svg> : index + 1}</span><span className="wb-setup-guide__step-copy"><strong>{labels[stage]}</strong></span></button>)}</nav>
    <div className="wb-setup-guide__content"><h3>{labels[currentStep]}</h3><p>{stages[currentStep].detail}</p>{currentStepContent}{stepIndex > 0 && <button type="button" className="wb-secondary" onClick={() => onStepChange(setupGuideStages[stepIndex - 1])}>Previous step</button>}</div>
  </section>;
}
